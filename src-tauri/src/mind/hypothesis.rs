use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

use super::canvas_db::{AppState, NodeInput};
use super::library_groups::{self, LibraryItemGroupSnapshot};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinkedDefinition {
    pub library_group_id: String,
    pub library_version_id: String,
    pub definition_name: String,
    pub version_string: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HypothesisInsight {
    pub id: String,
    pub hypothesis_id: String,
    pub content: String,
    pub created_at: i64,
}

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_notes: Option<String>,
    pub verdict: Option<String>,
    pub evaluated_at: Option<i64>,
    pub promoted_to_library_id: Option<String>,
    pub parent_hypothesis_id: Option<String>,
    #[serde(default)]
    pub linked_definitions: Vec<LinkedDefinition>,
    pub protocol_steps: Option<String>,
    pub protocol_locked_at: Option<i64>,
    pub raw_data: Option<String>,
    pub data_analysis: Option<String>,
    pub executed_at: Option<i64>,
    pub boundary_conditions: Option<String>,
    pub refined_from_id: Option<String>,
}

const HYPOTHESIS_COLUMNS: &str = "id, inquiry_id, statement_h1, statement_h0, \
    independent_var, dependent_var, metric, success_threshold, status, created_at, updated_at, \
    evidence_notes, verdict, evaluated_at, promoted_to_library_id, parent_hypothesis_id, \
    protocol_steps, protocol_locked_at, raw_data, data_analysis, executed_at, \
    boundary_conditions, refined_from_id";

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
        ("promoted_to_library_id", "TEXT NULL"),
        ("parent_hypothesis_id", "TEXT NULL"),
        ("protocol_steps", "TEXT NULL"),
        ("protocol_locked_at", "INTEGER NULL"),
        ("raw_data", "TEXT NULL"),
        ("data_analysis", "TEXT NULL"),
        ("executed_at", "INTEGER NULL"),
        ("boundary_conditions", "TEXT NULL"),
        ("refined_from_id", "TEXT NULL"),
    ] {
        if !columns.iter().any(|column| column == name) {
            conn.execute_batch(&format!(
                "ALTER TABLE hypotheses ADD COLUMN {name} {definition}"
            ))?;
        }
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS hypothesis_definitions (
            hypothesis_id TEXT NOT NULL,
            library_group_id TEXT NOT NULL,
            library_version_id TEXT NOT NULL,
            definition_name TEXT NOT NULL,
            version_string TEXT NOT NULL,
            PRIMARY KEY (hypothesis_id, library_group_id),
            FOREIGN KEY(hypothesis_id) REFERENCES hypotheses(id) ON DELETE CASCADE,
            FOREIGN KEY(library_group_id) REFERENCES library_item_groups(id),
            FOREIGN KEY(library_version_id) REFERENCES library_item_group_versions(id)
        );
        CREATE TABLE IF NOT EXISTS hypothesis_insights (
            id TEXT PRIMARY KEY,
            hypothesis_id TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(hypothesis_id) REFERENCES hypotheses(id) ON DELETE CASCADE
        );",
    )?;
    Ok(())
}

fn add_hypothesis_insight_in_db(
    conn: &Connection,
    id: &str,
    hypothesis_id: &str,
    content: &str,
    created_at: i64,
) -> std::result::Result<HypothesisInsight, String> {
    if content.trim().is_empty() {
        return Err("Insight content cannot be empty.".to_string());
    }
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM hypotheses WHERE id = ?1)",
            [hypothesis_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err(format!("Hypothesis not found: {hypothesis_id}"));
    }
    conn.execute(
        "INSERT INTO hypothesis_insights (id, hypothesis_id, content, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![id, hypothesis_id, content, created_at],
    )
    .map_err(|error| error.to_string())?;
    Ok(HypothesisInsight {
        id: id.to_string(),
        hypothesis_id: hypothesis_id.to_string(),
        content: content.to_string(),
        created_at,
    })
}

fn list_hypothesis_insights(
    conn: &Connection,
    hypothesis_id: &str,
) -> std::result::Result<Vec<HypothesisInsight>, String> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM hypotheses WHERE id = ?1)",
            [hypothesis_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err(format!("Hypothesis not found: {hypothesis_id}"));
    }
    let mut statement = conn
        .prepare(
            "SELECT id, hypothesis_id, content, created_at
             FROM hypothesis_insights
             WHERE hypothesis_id = ?1
             ORDER BY created_at DESC, id DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([hypothesis_id], |row| {
            Ok(HypothesisInsight {
                id: row.get(0)?,
                hypothesis_id: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

fn delete_hypothesis_insight_in_db(conn: &Connection, id: &str) -> std::result::Result<(), String> {
    let deleted = conn
        .execute("DELETE FROM hypothesis_insights WHERE id = ?1", [id])
        .map_err(|error| error.to_string())?;
    if deleted == 0 {
        return Err(format!("Hypothesis insight not found: {id}"));
    }
    Ok(())
}

#[tauri::command]
pub fn add_hypothesis_insight(
    state: State<AppState>,
    hypothesis_id: String,
    content: String,
) -> std::result::Result<HypothesisInsight, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|error| error.to_string())?;
    let id = format!("insight_{}", uuid::Uuid::now_v7());
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    add_hypothesis_insight_in_db(&conn, &id, &hypothesis_id, &content, now)
}

#[tauri::command]
pub fn get_hypothesis_insights(
    state: State<AppState>,
    hypothesis_id: String,
) -> std::result::Result<Vec<HypothesisInsight>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    list_hypothesis_insights(&conn, &hypothesis_id)
}

#[tauri::command]
pub fn delete_hypothesis_insight(
    state: State<AppState>,
    id: String,
) -> std::result::Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    delete_hypothesis_insight_in_db(&conn, &id)
}

fn insert_linked_definitions(
    transaction: &rusqlite::Transaction<'_>,
    hypothesis_id: &str,
    linked_definitions: &[LinkedDefinition],
) -> Result<()> {
    let mut linked_groups = HashSet::new();
    for definition in linked_definitions {
        if !linked_groups.insert(definition.library_group_id.as_str()) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let stored_version: Option<String> = transaction
            .query_row(
                "SELECT version FROM library_item_group_versions
                 WHERE id = ?1 AND group_id = ?2",
                params![
                    definition.library_version_id,
                    definition.library_group_id
                ],
                |row| row.get(0),
            )
            .optional()?;
        if stored_version.as_deref() != Some(definition.version_string.as_str()) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        transaction.execute(
            "INSERT INTO hypothesis_definitions (
                hypothesis_id, library_group_id, library_version_id,
                definition_name, version_string
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                hypothesis_id,
                definition.library_group_id,
                definition.library_version_id,
                definition.definition_name,
                definition.version_string
            ],
        )?;
    }
    Ok(())
}

fn list_linked_definitions(conn: &Connection, hypothesis_id: &str) -> Result<Vec<LinkedDefinition>> {
    let mut statement = conn.prepare(
        "SELECT library_group_id, library_version_id, definition_name, version_string
         FROM hypothesis_definitions
         WHERE hypothesis_id = ?1
         ORDER BY definition_name COLLATE NOCASE, library_group_id",
    )?;
    let rows = statement.query_map([hypothesis_id], |row| {
        Ok(LinkedDefinition {
            library_group_id: row.get(0)?,
            library_version_id: row.get(1)?,
            definition_name: row.get(2)?,
            version_string: row.get(3)?,
        })
    })?;
    rows.collect()
}

fn create_child_hypothesis_in_db(
    conn: &mut Connection,
    parent_id: &str,
    id: &str,
    statement_h1: &str,
    statement_h0: &str,
    independent_var: &str,
    dependent_var: &str,
    metric: &str,
    success_threshold: &str,
    linked_definitions: &[LinkedDefinition],
    now: i64,
) -> std::result::Result<Hypothesis, String> {
    let transaction = conn.transaction().map_err(|error| error.to_string())?;
    let (inquiry_id, parent_verdict): (String, Option<String>) = transaction
        .query_row(
            "SELECT inquiry_id, verdict FROM hypotheses WHERE id = ?1",
            [parent_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Parent hypothesis not found: {parent_id}"))?;
    if parent_verdict.as_deref().is_some_and(|verdict| {
        !matches!(verdict, "rejected" | "supported" | "partially_supported")
    }) {
        return Err("A child hypothesis can only be created from a rejected, supported, or partially supported hypothesis.".to_string());
    }
    let refined_from_id = matches!(
        parent_verdict.as_deref(),
        Some("supported" | "partially_supported")
    )
    .then_some(parent_id);

    transaction
        .execute(
            "INSERT INTO hypotheses (
                id, inquiry_id, statement_h1, statement_h0, independent_var,
                dependent_var, metric, success_threshold, status, created_at,
                updated_at, parent_hypothesis_id, refined_from_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'draft', ?9, ?9, ?10, ?11)",
            params![
                id,
                inquiry_id,
                statement_h1,
                statement_h0,
                independent_var,
                dependent_var,
                metric,
                success_threshold,
                now,
                parent_id,
                refined_from_id
            ],
        )
        .map_err(|error| error.to_string())?;
    insert_linked_definitions(&transaction, id, linked_definitions)
        .map_err(|error| error.to_string())?;
    let hypothesis = query_hypothesis(&transaction, id).map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(hypothesis)
}

#[tauri::command]
pub fn create_child_hypothesis(
    state: State<AppState>,
    parent_id: String,
    statement_h1: String,
    statement_h0: String,
    independent_var: String,
    dependent_var: String,
    metric: String,
    success_threshold: String,
    linked_definitions: Option<Vec<LinkedDefinition>>,
) -> Result<Hypothesis, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|error| error.to_string())?;
    let id = format!("hypothesis_{}", uuid::Uuid::now_v7());
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_child_hypothesis_in_db(
        &mut conn,
        &parent_id,
        &id,
        &statement_h1,
        &statement_h0,
        &independent_var,
        &dependent_var,
        &metric,
        &success_threshold,
        linked_definitions.as_deref().unwrap_or_default(),
        now,
    )
}

fn hypothesis_library_node(hypothesis: &Hypothesis) -> NodeInput {
    let boundary_conditions = hypothesis.boundary_conditions.as_deref();
    let title = if hypothesis.verdict.as_deref() == Some("partially_supported") {
        format!(
            "{} (Valid only in {})",
            hypothesis.statement_h1,
            boundary_conditions.unwrap_or("specified conditions")
        )
    } else {
        hypothesis.statement_h1.clone()
    };
    let content = format!(
        "H1: {}\n\nH0: {}\n\nIndependent variable: {}\nDependent variable: {}\nMetric: {}\nSuccess threshold: {}\n\nBoundary conditions: {}\n\nEvidence notes: {}",
        hypothesis.statement_h1,
        hypothesis.statement_h0,
        hypothesis.independent_var,
        hypothesis.dependent_var,
        hypothesis.metric,
        hypothesis.success_threshold,
        boundary_conditions.unwrap_or("None recorded"),
        hypothesis.evidence_notes.as_deref().unwrap_or("None recorded")
    );
    NodeInput {
        id: format!("hypothesis_{}", hypothesis.id),
        node_type: "voraText".to_string(),
        title: title.clone(),
        content: content.clone(),
        version: "v0.1.0".to_string(),
        data: serde_json::json!({
            "text": content,
            "hypothesisId": hypothesis.id,
            "statementH1": hypothesis.statement_h1,
            "statementH0": hypothesis.statement_h0,
            "independentVar": hypothesis.independent_var,
            "dependentVar": hypothesis.dependent_var,
            "metric": hypothesis.metric,
            "successThreshold": hypothesis.success_threshold,
            "evidenceNotes": hypothesis.evidence_notes,
            "boundaryConditions": hypothesis.boundary_conditions,
            "refinedFromId": hypothesis.refined_from_id,
        }),
        x: 0.0,
        y: 0.0,
        width: Some(360.0),
        height: Some(240.0),
        style: None,
    }
}

fn graduate_to_library_in_db(
    conn: &mut Connection,
    hypothesis_id: &str,
    library_group_id: Option<&str>,
    now: i64,
) -> std::result::Result<Hypothesis, String> {
    let transaction = conn.transaction().map_err(|error| error.to_string())?;
    let hypothesis =
        query_hypothesis(&transaction, hypothesis_id).map_err(|error| error.to_string())?;
    if !matches!(
        hypothesis.verdict.as_deref(),
        Some("supported" | "partially_supported")
    ) {
        return Err("Only supported or partially supported hypotheses can be graduated to the Library".to_string());
    }
    if hypothesis.promoted_to_library_id.is_some() {
        return Err("Hypothesis is already graduated to the Library".to_string());
    }

    let node = hypothesis_library_node(&hypothesis);
    let library_title = node.title.clone();
    let group_id = if let Some(group_id) = library_group_id {
        let (version, snapshot_json, deleted_at): (String, String, Option<i64>) = transaction
            .query_row(
                "SELECT version, snapshot_json, deleted_at
                 FROM library_item_groups WHERE id = ?1",
                [group_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|error| error.to_string())?;
        if deleted_at.is_some() {
            return Err(format!("Library item group is deleted: {group_id}"));
        }
        let mut snapshot: LibraryItemGroupSnapshot =
            serde_json::from_str(&snapshot_json).map_err(|error| error.to_string())?;
        snapshot.nodes.push(node);
        let (major, minor, patch) = super::canvas_db::parse_board_version(&version)
            .ok_or_else(|| format!("Invalid Library item group version: {version}"))?;
        let next_patch = patch
            .checked_add(1)
            .ok_or_else(|| "Library item group version is exhausted".to_string())?;
        let next_version = format!("{major}.{minor}.{next_patch}");
        library_groups::commit_item_group_version_in_transaction(
            &transaction,
            group_id,
            &next_version,
            snapshot,
            now,
        )
        .map_err(|error| error.to_string())?;
        group_id.to_string()
    } else {
        let name = if hypothesis.statement_h1.trim().is_empty() {
            "Supported hypothesis".to_string()
        } else {
            library_title.chars().take(80).collect()
        };
        library_groups::create_item_group_in_transaction(
            &transaction,
            &name,
            "0.1.0",
            LibraryItemGroupSnapshot {
                nodes: vec![node],
                edges: Vec::new(),
                cards: Vec::new(),
            },
            now,
        )
        .map_err(|error| error.to_string())?
        .id
    };

    let changed = transaction
        .execute(
            "UPDATE hypotheses
             SET promoted_to_library_id = ?1, updated_at = ?2
             WHERE id = ?3 AND verdict IN ('supported', 'partially_supported')
               AND promoted_to_library_id IS NULL",
            params![group_id, now, hypothesis_id],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err(format!("Could not graduate hypothesis: {hypothesis_id}"));
    }
    let updated =
        query_hypothesis(&transaction, hypothesis_id).map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(updated)
}

#[tauri::command]
pub fn graduate_to_library(
    state: State<AppState>,
    hypothesis_id: String,
    library_group_id: Option<String>,
) -> Result<Hypothesis, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|error| error.to_string())?;
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    graduate_to_library_in_db(&mut conn, &hypothesis_id, library_group_id.as_deref(), now)
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
    linked_definitions: &[LinkedDefinition],
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
    insert_linked_definitions(&transaction, id, linked_definitions)?;

    let hypothesis = query_hypothesis(&transaction, id)?;
    transaction.commit()?;
    Ok(hypothesis)
}

pub fn list_hypotheses(conn: &Connection) -> Result<Vec<Hypothesis>> {
    let mut statement = conn.prepare(&format!(
        "SELECT {HYPOTHESIS_COLUMNS} FROM hypotheses ORDER BY created_at DESC"
    ))?;
    let rows = statement.query_map([], map_hypothesis)?;
    let mut hypotheses = rows.collect::<Result<Vec<_>>>()?;
    for hypothesis in &mut hypotheses {
        hypothesis.linked_definitions = list_linked_definitions(conn, &hypothesis.id)?;
    }
    Ok(hypotheses)
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
    linked_definitions: Option<Vec<LinkedDefinition>>,
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
        linked_definitions.as_deref().unwrap_or_default(),
        now,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_hypotheses(state: State<AppState>) -> Result<Vec<Hypothesis>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    list_hypotheses(&conn).map_err(|error| error.to_string())
}

fn save_hypothesis_protocol_in_db(
    conn: &mut Connection,
    id: &str,
    protocol_steps: &str,
    now: i64,
) -> std::result::Result<Hypothesis, String> {
    if protocol_steps.trim().is_empty() {
        return Err("Protocol steps cannot be empty.".to_string());
    }

    let transaction = conn.transaction().map_err(|error| error.to_string())?;
    let existing: Option<(Option<String>, Option<i64>, Option<i64>)> = transaction
        .query_row(
            "SELECT protocol_steps, protocol_locked_at, executed_at
             FROM hypotheses WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let (existing_steps, locked_at, executed_at) =
        existing.ok_or_else(|| format!("Hypothesis not found: {id}"))?;

    if let Some(existing_steps) = existing_steps {
        if existing_steps == protocol_steps {
            return query_hypothesis(&transaction, id).map_err(|error| error.to_string());
        }
        return Err(if executed_at.is_some() {
            "Protocol steps cannot be changed after test execution.".to_string()
        } else {
            "Protocol steps are locked and cannot be changed.".to_string()
        });
    }
    if locked_at.is_some() || executed_at.is_some() {
        return Err("Protocol steps cannot be changed after test execution.".to_string());
    }

    transaction
        .execute(
            "UPDATE hypotheses
             SET protocol_steps = ?1, protocol_locked_at = ?2, updated_at = ?2
             WHERE id = ?3 AND protocol_locked_at IS NULL AND executed_at IS NULL",
            params![protocol_steps, now, id],
        )
        .map_err(|error| error.to_string())?;
    let hypothesis = query_hypothesis(&transaction, id).map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(hypothesis)
}

#[tauri::command]
pub fn save_hypothesis_protocol(
    state: State<AppState>,
    id: String,
    protocol_steps: String,
) -> std::result::Result<Hypothesis, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|error| error.to_string())?;
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    save_hypothesis_protocol_in_db(&mut conn, &id, &protocol_steps, now)
}

fn record_hypothesis_execution_in_db(
    conn: &mut Connection,
    id: &str,
    raw_data: &str,
    now: i64,
) -> std::result::Result<Hypothesis, String> {
    let changed = conn
        .execute(
            "UPDATE hypotheses
             SET raw_data = ?1, executed_at = ?2, updated_at = ?2
             WHERE id = ?3 AND protocol_locked_at IS NOT NULL
               AND protocol_steps IS NOT NULL AND length(trim(protocol_steps)) > 0
               AND executed_at IS NULL",
            params![raw_data, now, id],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        let state: Option<(Option<i64>, Option<i64>)> = conn
            .query_row(
                "SELECT protocol_locked_at, executed_at FROM hypotheses WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        return match state {
            None => Err(format!("Hypothesis not found: {id}")),
            Some((_, Some(_))) => Err("Test execution has already been recorded.".to_string()),
            Some(_) => Err("Save and lock the protocol before recording execution.".to_string()),
        };
    }
    query_hypothesis(conn, id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn record_hypothesis_execution(
    state: State<AppState>,
    id: String,
    raw_data: String,
) -> std::result::Result<Hypothesis, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|error| error.to_string())?;
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    record_hypothesis_execution_in_db(&mut conn, &id, &raw_data, now)
}

fn record_verdict_in_db(
    conn: &Connection,
    id: &str,
    evidence_notes: &str,
    verdict: &str,
    raw_data: Option<&str>,
    data_analysis: Option<&str>,
    boundary_conditions: Option<&str>,
    now: i64,
) -> std::result::Result<Hypothesis, String> {
    if !matches!(
        verdict,
        "supported" | "partially_supported" | "rejected" | "inconclusive"
    ) {
        return Err(format!("Invalid hypothesis verdict: {verdict}"));
    }
    if verdict == "partially_supported"
        && boundary_conditions.map_or(true, |conditions| conditions.trim().is_empty())
    {
        return Err("Boundary conditions are required for a partially supported verdict.".to_string());
    }
    let stored_raw_data: Option<String> = conn
        .query_row("SELECT raw_data FROM hypotheses WHERE id = ?1", [id], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("Hypothesis not found: {id}"))?;
    if raw_data.is_some_and(|submitted| Some(submitted) != stored_raw_data.as_deref()) {
        return Err("Raw data cannot be changed after test execution.".to_string());
    }
    let changed = conn
        .execute(
            "UPDATE hypotheses
             SET evidence_notes = ?1, verdict = ?2, data_analysis = COALESCE(?3, data_analysis),
                 boundary_conditions = ?4, evaluated_at = ?5, updated_at = ?5
             WHERE id = ?6 AND executed_at IS NOT NULL",
            params![evidence_notes, verdict, data_analysis, boundary_conditions, now, id],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err("Record test execution before saving a verdict.".to_string());
    }
    query_hypothesis(conn, id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn record_verdict(
    state: State<AppState>,
    id: String,
    evidence_notes: String,
    verdict: String,
    raw_data: Option<String>,
    data_analysis: Option<String>,
    boundary_conditions: Option<String>,
) -> std::result::Result<Hypothesis, String> {
    if !matches!(
        verdict.as_str(),
        "supported" | "partially_supported" | "rejected" | "inconclusive"
    ) {
        return Err(format!("Invalid hypothesis verdict: {verdict}"));
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|error| error.to_string())?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    record_verdict_in_db(
        &conn,
        &id,
        &evidence_notes,
        &verdict,
        raw_data.as_deref(),
        data_analysis.as_deref(),
        boundary_conditions.as_deref(),
        now,
    )
}

fn query_hypothesis(conn: &Connection, id: &str) -> Result<Hypothesis> {
    let sql = format!("SELECT {HYPOTHESIS_COLUMNS} FROM hypotheses WHERE id = ?1");
    let mut hypothesis = conn.query_row(&sql, [id], map_hypothesis)?;
    hypothesis.linked_definitions = list_linked_definitions(conn, id)?;
    Ok(hypothesis)
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
        promoted_to_library_id: row.get(14)?,
        parent_hypothesis_id: row.get(15)?,
        linked_definitions: Vec::new(),
        protocol_steps: row.get(16)?,
        protocol_locked_at: row.get(17)?,
        raw_data: row.get(18)?,
        data_analysis: row.get(19)?,
        executed_at: row.get(20)?,
        boundary_conditions: row.get(21)?,
        refined_from_id: row.get(22)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        add_hypothesis_insight_in_db, create_child_hypothesis_in_db,
        delete_hypothesis_insight_in_db, graduate_to_library_in_db, init_schema,
        list_hypothesis_insights, list_hypotheses, promote_inquiry,
        record_hypothesis_execution_in_db, record_verdict_in_db,
        save_hypothesis_protocol_in_db, LinkedDefinition,
    };
    use crate::mind::questions::{
        create_question, init_schema as init_questions_schema, select_for_promotion,
        triage_question,
    };
    use rusqlite::Connection;

    fn setup() -> Connection {
        let connection = Connection::open_in_memory().expect("open in-memory database");
        crate::mind::canvas_db::init_db(&connection).expect("initialize Library schema");
        init_questions_schema(&connection).expect("initialize questions schema");
        init_schema(&connection).expect("initialize hypotheses schema");
        init_schema(&connection).expect("reinitialize hypotheses schema");
        connection
    }

    #[test]
    fn insights_are_hypothesis_scoped_newest_first_and_deletable() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        promote(&mut connection, "inquiry-1", "hypothesis-1", 10).expect("promote inquiry");

        add_hypothesis_insight_in_db(
            &connection,
            "insight-old",
            "hypothesis-1",
            "First observation",
            20,
        )
        .expect("add first insight");
        let newest = add_hypothesis_insight_in_db(
            &connection,
            "insight-new",
            "hypothesis-1",
            "Follow-up idea",
            30,
        )
        .expect("add second insight");

        assert_eq!(newest.content, "Follow-up idea");
        assert_eq!(
            list_hypothesis_insights(&connection, "hypothesis-1")
                .expect("list hypothesis insights")
                .iter()
                .map(|insight| insight.id.as_str())
                .collect::<Vec<_>>(),
            vec!["insight-new", "insight-old"]
        );
        assert!(add_hypothesis_insight_in_db(
            &connection,
            "insight-empty",
            "hypothesis-1",
            "  ",
            40,
        )
        .is_err());
        assert!(list_hypothesis_insights(&connection, "missing").is_err());

        delete_hypothesis_insight_in_db(&connection, "insight-new").expect("delete insight");
        assert_eq!(
            list_hypothesis_insights(&connection, "hypothesis-1")
                .expect("list remaining insights")
                .iter()
                .map(|insight| insight.id.as_str())
                .collect::<Vec<_>>(),
            vec!["insight-old"]
        );
        assert!(delete_hypothesis_insight_in_db(&connection, "missing").is_err());
    }

    fn promote(
        conn: &mut Connection,
        inquiry_id: &str,
        id: &str,
        now: i64,
    ) -> rusqlite::Result<super::Hypothesis> {
        promote_with_definitions(conn, inquiry_id, id, &[], now)
    }

    fn promote_with_definitions(
        conn: &mut Connection,
        inquiry_id: &str,
        id: &str,
        linked_definitions: &[LinkedDefinition],
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
            linked_definitions,
            now,
        )
    }

    fn insert_test_definition(conn: &Connection) -> LinkedDefinition {
        conn.execute(
            "INSERT INTO library_item_groups
                (id, name, version, snapshot_json, created_at, updated_at, deleted_at)
             VALUES ('definition-1', 'Flag', '0.1.0', '{}', 1, 1, NULL)",
            [],
        )
        .expect("insert Library definition");
        conn.execute(
            "INSERT INTO library_item_group_versions
                (id, group_id, version, snapshot_json, created_at)
             VALUES ('definition-version-1', 'definition-1', '0.1.0', '{}', 1)",
            [],
        )
        .expect("insert Library definition version");
        LinkedDefinition {
            library_group_id: "definition-1".to_string(),
            library_version_id: "definition-version-1".to_string(),
            definition_name: "Flag".to_string(),
            version_string: "0.1.0".to_string(),
        }
    }

    fn select_inquiry(conn: &mut Connection, inquiry_id: &str) {
        create_question(conn, inquiry_id, "A question", "", 1).expect("create inquiry");
        triage_question(conn, inquiry_id, 3, 3, 3, None, 2).expect("triage inquiry");
        select_for_promotion(conn, inquiry_id, 3).expect("select inquiry");
    }

    fn create_supported_hypothesis(conn: &mut Connection, inquiry_id: &str, id: &str) {
        select_inquiry(conn, inquiry_id);
        promote(conn, inquiry_id, id, 10).expect("promote inquiry");
        save_hypothesis_protocol_in_db(conn, id, "Run the controlled test.", 15)
            .expect("save test protocol");
        record_hypothesis_execution_in_db(conn, id, "Raw observations", 18)
            .expect("record test execution");
        record_verdict_in_db(
            conn,
            id,
            "Evidence supports H1",
            "supported",
            Some("Raw observations"),
            Some("Results support H1."),
            None,
            20,
        )
            .expect("record supported verdict");
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
    fn promotion_persists_and_returns_linked_definition_versions() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        let definition = insert_test_definition(&connection);

        let hypothesis = promote_with_definitions(
            &mut connection,
            "inquiry-1",
            "hypothesis-1",
            std::slice::from_ref(&definition),
            10,
        )
        .expect("promote hypothesis with linked definition");

        assert_eq!(hypothesis.linked_definitions, vec![definition.clone()]);
        assert_eq!(
            list_hypotheses(&connection).expect("list hypotheses")[0].linked_definitions,
            vec![definition]
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
        save_hypothesis_protocol_in_db(&mut connection, "hypothesis-1", "Run the test.", 15)
            .expect("save test protocol");
        record_hypothesis_execution_in_db(&mut connection, "hypothesis-1", "Raw observations", 18)
            .expect("record test execution");

        let hypothesis = record_verdict_in_db(
            &connection,
            "hypothesis-1",
            "Observed results matched H1",
            "supported",
            Some("Raw observations"),
            Some("Analysis supports H1."),
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
        assert_eq!(
            hypothesis.data_analysis.as_deref(),
            Some("Analysis supports H1.")
        );
        assert_eq!(hypothesis.executed_at, Some(18));

        let updated_hypothesis = record_verdict_in_db(
            &connection,
            "hypothesis-1",
            "New evidence contradicts H1",
            "rejected",
            Some("Raw observations"),
            Some("Reanalysis contradicts H1."),
            None,
            30,
        )
        .expect("update verdict");
        assert_eq!(
            updated_hypothesis.evidence_notes.as_deref(),
            Some("New evidence contradicts H1")
        );
        assert_eq!(updated_hypothesis.verdict.as_deref(), Some("rejected"));
        assert_eq!(updated_hypothesis.evaluated_at, Some(30));
        assert_eq!(updated_hypothesis.updated_at, 30);
        assert_eq!(
            updated_hypothesis.data_analysis.as_deref(),
            Some("Reanalysis contradicts H1.")
        );

        assert!(record_verdict_in_db(
            &connection,
            "hypothesis-1",
            "Invalid verdict attempt",
            "maybe",
            None,
            None,
            None,
            40,
        )
        .is_err());
    }

    #[test]
    fn protocol_must_be_locked_before_execution_and_cannot_change_afterwards() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        promote(&mut connection, "inquiry-1", "hypothesis-1", 10).expect("promote inquiry");

        assert!(record_hypothesis_execution_in_db(
            &mut connection,
            "hypothesis-1",
            "Raw observations",
            15,
        )
        .is_err());

        let saved = save_hypothesis_protocol_in_db(
            &mut connection,
            "hypothesis-1",
            "Record ten observations.",
            20,
        )
        .expect("save protocol");
        assert_eq!(
            saved.protocol_steps.as_deref(),
            Some("Record ten observations.")
        );
        assert_eq!(saved.protocol_locked_at, Some(20));

        let executed = record_hypothesis_execution_in_db(
            &mut connection,
            "hypothesis-1",
            "Ten observations",
            30,
        )
        .expect("record execution");
        assert_eq!(executed.raw_data.as_deref(), Some("Ten observations"));
        assert_eq!(executed.executed_at, Some(30));

        assert!(
            save_hypothesis_protocol_in_db(
                &mut connection,
                "hypothesis-1",
                "Changed protocol",
                40,
            )
            .is_err()
        );
    }

    #[test]
    fn verdict_requires_execution_and_raw_data_cannot_be_changed_after_execution() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        promote(&mut connection, "inquiry-1", "hypothesis-1", 10).expect("promote inquiry");
        assert!(record_verdict_in_db(
            &connection,
            "hypothesis-1",
            "Premature verdict",
            "supported",
            None,
            None,
            None,
            15,
        )
        .is_err());

        save_hypothesis_protocol_in_db(&mut connection, "hypothesis-1", "Run the test.", 20)
            .expect("save protocol");
        record_hypothesis_execution_in_db(&mut connection, "hypothesis-1", "Original data", 30)
            .expect("record execution");
        assert!(record_verdict_in_db(
            &connection,
            "hypothesis-1",
            "Evidence",
            "supported",
            Some("Altered data"),
            None,
            None,
            40,
        )
        .is_err());
    }

    #[test]
    fn graduation_creates_a_library_group_and_links_the_hypothesis_atomically() {
        let mut connection = setup();
        create_supported_hypothesis(&mut connection, "inquiry-1", "hypothesis-1");

        let hypothesis =
            graduate_to_library_in_db(&mut connection, "hypothesis-1", None, 30)
                .expect("graduate hypothesis");
        let group_id = hypothesis
            .promoted_to_library_id
            .as_deref()
            .expect("linked Library group");
        let (group_count, version_count, name, snapshot_json): (i64, i64, String, String) =
            connection
                .query_row(
                    "SELECT
                        (SELECT COUNT(*) FROM library_item_groups WHERE id = ?1),
                        (SELECT COUNT(*) FROM library_item_group_versions WHERE group_id = ?1),
                        (SELECT name FROM library_item_groups WHERE id = ?1),
                        (SELECT snapshot_json FROM library_item_groups WHERE id = ?1)",
                    [group_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .expect("read Library group");
        assert_eq!(group_count, 1);
        assert_eq!(version_count, 1);
        assert_eq!(name, "If X then Y");
        let snapshot: serde_json::Value =
            serde_json::from_str(&snapshot_json).expect("parse Library snapshot");
        let content = snapshot["nodes"][0]["content"]
            .as_str()
            .expect("hypothesis content");
        assert!(content.contains("Evidence notes: Evidence supports H1"));
        assert!(content.contains("Independent variable: X"));

        assert!(graduate_to_library_in_db(&mut connection, "hypothesis-1", None, 40).is_err());
    }

    #[test]
    fn graduation_appends_to_an_existing_library_group_version() {
        let mut connection = setup();
        create_supported_hypothesis(&mut connection, "inquiry-1", "hypothesis-1");
        let first = graduate_to_library_in_db(&mut connection, "hypothesis-1", None, 30)
            .expect("graduate first hypothesis");
        let group_id = first.promoted_to_library_id.expect("first Library group");

        create_supported_hypothesis(&mut connection, "inquiry-2", "hypothesis-2");
        let second = graduate_to_library_in_db(
            &mut connection,
            "hypothesis-2",
            Some(&group_id),
            40,
        )
        .expect("append second hypothesis");

        assert_eq!(second.promoted_to_library_id.as_deref(), Some(group_id.as_str()));
        let (version, versions, nodes): (String, i64, i64) = connection
            .query_row(
                "SELECT groups.version,
                        (SELECT COUNT(*) FROM library_item_group_versions WHERE group_id = groups.id),
                        json_array_length(groups.snapshot_json, '$.nodes')
                 FROM library_item_groups groups WHERE groups.id = ?1",
                [&group_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read appended Library group");
        assert_eq!(version, "0.1.1");
        assert_eq!(versions, 2);
        assert_eq!(nodes, 2);
    }

    #[test]
    fn graduation_requires_a_supported_verdict() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        promote(&mut connection, "inquiry-1", "hypothesis-1", 10).expect("promote inquiry");

        assert!(graduate_to_library_in_db(&mut connection, "hypothesis-1", None, 20).is_err());
        let groups: i64 = connection
            .query_row("SELECT COUNT(*) FROM library_item_groups", [], |row| row.get(0))
            .expect("count Library groups");
        assert_eq!(groups, 0);
    }

    #[test]
    fn child_hypothesis_inherits_inquiry_and_records_parent() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        let definition = insert_test_definition(&connection);
        let parent = promote_with_definitions(
            &mut connection,
            "inquiry-1",
            "hypothesis-1",
            std::slice::from_ref(&definition),
            10,
        )
        .expect("promote inquiry");

        let child = create_child_hypothesis_in_db(
            &mut connection,
            "hypothesis-1",
            "hypothesis-child",
            "If X over a longer timeframe then Y",
            "X does not affect Y",
            "X over a longer timeframe",
            "Y",
            "Y per day",
            "Y increases by 10%",
            &parent.linked_definitions,
            20,
        )
        .expect("create child hypothesis");

        assert_eq!(child.status, "draft");
        assert_eq!(child.inquiry_id, "inquiry-1");
        assert_eq!(child.parent_hypothesis_id.as_deref(), Some("hypothesis-1"));
        assert_eq!(child.created_at, 20);
        assert_eq!(child.linked_definitions, vec![definition]);
    }
}
