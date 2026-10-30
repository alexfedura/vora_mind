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
pub struct DataEntry {
    pub id: String,
    pub hypothesis_id: String,
    pub data_json: String,
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
    #[serde(default)]
    pub linked_snapshots: Vec<String>,
    pub protocol_steps: Option<String>,
    pub protocol_locked_at: Option<i64>,
    pub raw_data: Option<String>,
    pub data_analysis: Option<String>,
    pub executed_at: Option<i64>,
    pub boundary_conditions: Option<String>,
    pub refined_from_id: Option<String>,
    pub checklist_schema: Option<String>,
    pub protocol_type: String,
    pub protocol_flowchart_json: Option<String>,
    pub protocol_completed_steps: Option<String>,
}

const HYPOTHESIS_COLUMNS: &str = "id, inquiry_id, statement_h1, statement_h0, \
    independent_var, dependent_var, metric, success_threshold, status, created_at, updated_at, \
    evidence_notes, verdict, evaluated_at, promoted_to_library_id, parent_hypothesis_id, \
    protocol_steps, protocol_locked_at, raw_data, data_analysis, executed_at, \
    boundary_conditions, refined_from_id, checklist_schema, protocol_type, \
    protocol_flowchart_json, protocol_completed_steps";

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
        ("checklist_schema", "TEXT NULL"),
        ("protocol_type", "TEXT NOT NULL DEFAULT 'text'"),
        ("protocol_flowchart_json", "TEXT NULL"),
        ("protocol_completed_steps", "TEXT NULL"),
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
        );
        CREATE TABLE IF NOT EXISTS hypothesis_data_entries (
            id TEXT PRIMARY KEY,
            hypothesis_id TEXT NOT NULL,
            data_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(hypothesis_id) REFERENCES hypotheses(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_hypothesis_data_entries_hypothesis
            ON hypothesis_data_entries(hypothesis_id, created_at, id);
        CREATE TABLE IF NOT EXISTS hypothesis_snapshots (
            hypothesis_id TEXT NOT NULL,
            snapshot_id TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (hypothesis_id, snapshot_id)
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
                params![definition.library_version_id, definition.library_group_id],
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

fn list_linked_definitions(
    conn: &Connection,
    hypothesis_id: &str,
) -> Result<Vec<LinkedDefinition>> {
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
    if parent_verdict.is_none() {
        return Err(
            "A child hypothesis can only be created from an evaluated hypothesis.".to_string(),
        );
    }
    let refined_from_id = Some(parent_id);

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
    let statement_title = if hypothesis.statement_h1.trim().is_empty() {
        "Supported hypothesis"
    } else {
        hypothesis.statement_h1.as_str()
    };
    let title = boundary_conditions
        .filter(|conditions| !conditions.trim().is_empty())
        .map_or_else(
            || statement_title.to_string(),
            |conditions| format!("{statement_title} (Valid only when: {conditions})"),
        );
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
        return Err(
            "Only supported or partially supported hypotheses can be graduated to the Library"
                .to_string(),
        );
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
        let name = library_title;
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
        hypothesis.linked_snapshots = list_hypothesis_snapshots(conn, &hypothesis.id)?;
    }
    Ok(hypotheses)
}

fn list_hypothesis_snapshots(conn: &Connection, hypothesis_id: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT snapshot_id FROM hypothesis_snapshots
         WHERE hypothesis_id = ?1 ORDER BY created_at, snapshot_id",
    )?;
    let rows = statement.query_map([hypothesis_id], |row| row.get(0))?;
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

fn validate_checklist_schema(schema_json: &str) -> std::result::Result<serde_json::Value, String> {
    let schema: serde_json::Value = serde_json::from_str(schema_json)
        .map_err(|error| format!("Checklist schema must be valid JSON: {error}"))?;
    let fields = schema
        .as_array()
        .ok_or_else(|| "Checklist schema must be a JSON array.".to_string())?;
    if fields.is_empty() {
        return Err("Checklist schema must contain at least one field.".to_string());
    }

    let mut labels = HashSet::new();
    for field in fields {
        let raw_label = field
            .get("label")
            .and_then(serde_json::Value::as_str)
            .filter(|label| !label.trim().is_empty())
            .ok_or_else(|| "Each checklist field needs a non-empty label.".to_string())?;
        let label = raw_label.trim();
        if raw_label != label {
            return Err(format!(
                "Checklist field labels cannot start or end with whitespace: {label}"
            ));
        }
        if !labels.insert(label.to_string()) {
            return Err(format!("Checklist field labels must be unique: {label}"));
        }
        let field_type = field
            .get("type")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("Checklist field '{label}' needs a type."))?;
        if !matches!(field_type, "select" | "number" | "text" | "boolean") {
            return Err(format!(
                "Unsupported checklist field type '{field_type}' for '{label}'."
            ));
        }
        if field_type == "select" {
            let options = field
                .get("options")
                .and_then(serde_json::Value::as_array)
                .filter(|options| !options.is_empty())
                .ok_or_else(|| {
                    format!("Select checklist field '{label}' needs at least one option.")
                })?;
            let mut unique_options = HashSet::new();
            for raw_option in options {
                let raw_option = raw_option
                    .as_str()
                    .filter(|option| !option.trim().is_empty())
                    .ok_or_else(|| {
                        format!("Select checklist field '{label}' has an empty option.")
                    })?;
                let option = raw_option.trim();
                if raw_option != option {
                    return Err(format!(
                        "Options for checklist field '{label}' cannot start or end with whitespace."
                    ));
                }
                if !unique_options.insert(option.to_string()) {
                    return Err(format!(
                        "Select checklist field '{label}' has a duplicate option: {option}"
                    ));
                }
            }
        }
    }
    Ok(schema)
}

fn validate_checklist_entry(schema_json: &str, data_json: &str) -> std::result::Result<(), String> {
    let schema = validate_checklist_schema(schema_json)?;
    let data: serde_json::Value = serde_json::from_str(data_json)
        .map_err(|error| format!("Checklist entry must be valid JSON: {error}"))?;
    let values = data
        .as_object()
        .ok_or_else(|| "Checklist entry must be a JSON object.".to_string())?;
    let fields = schema
        .as_array()
        .ok_or_else(|| "Checklist schema must be a JSON array.".to_string())?;
    if values.len() != fields.len() {
        return Err("Checklist entry must provide exactly one value for each field.".to_string());
    }

    for field in fields {
        let label = field
            .get("label")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "Checklist schema contains a field without a label.".to_string())?;
        let value = values
            .get(label)
            .ok_or_else(|| format!("Checklist entry is missing the '{label}' field."))?;
        let field_type = field
            .get("type")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("Checklist schema field '{label}' has no type."))?;
        let valid = match field_type {
            "select" => value.as_str().is_some_and(|value| {
                field
                    .get("options")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|options| {
                        options.iter().any(|option| option.as_str() == Some(value))
                    })
            }),
            "number" => value.as_number().is_some(),
            "text" => value.is_string(),
            "boolean" => value.is_boolean(),
            _ => false,
        };
        if !valid {
            return Err(format!(
                "Checklist entry value for '{label}' does not match type '{field_type}'."
            ));
        }
    }
    Ok(())
}

fn save_checklist_schema_in_db(
    conn: &Connection,
    hypothesis_id: &str,
    schema_json: &str,
    now: i64,
) -> std::result::Result<Hypothesis, String> {
    validate_checklist_schema(schema_json)?;
    let protocol_state: Option<(Option<i64>, Option<i64>)> = conn
        .query_row(
            "SELECT protocol_locked_at, executed_at FROM hypotheses WHERE id = ?1",
            [hypothesis_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((protocol_locked_at, executed_at)) = protocol_state else {
        return Err(format!("Hypothesis not found: {hypothesis_id}"));
    };
    if protocol_locked_at.is_none() {
        return Err("Lock the test protocol before defining a checklist.".to_string());
    }
    if executed_at.is_some() {
        return Err("Checklist schema cannot be changed after test execution.".to_string());
    }
    let entry_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM hypothesis_data_entries WHERE hypothesis_id = ?1",
            [hypothesis_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if entry_count > 0 {
        return Err(
            "Checklist schema cannot be changed after data entries have been collected."
                .to_string(),
        );
    }
    conn.execute(
        "UPDATE hypotheses SET checklist_schema = ?1, updated_at = ?2 WHERE id = ?3",
        params![schema_json, now, hypothesis_id],
    )
    .map_err(|error| error.to_string())?;
    query_hypothesis(conn, hypothesis_id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_checklist_schema(
    state: State<AppState>,
    hypothesis_id: String,
    schema_json: String,
) -> std::result::Result<Hypothesis, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|error| error.to_string())?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    save_checklist_schema_in_db(&conn, &hypothesis_id, &schema_json, now)
}

fn add_data_entry_in_db(
    conn: &Connection,
    id: &str,
    hypothesis_id: &str,
    data_json: &str,
    created_at: i64,
) -> std::result::Result<DataEntry, String> {
    let hypothesis_state: Option<(Option<String>, Option<i64>, Option<i64>)> = conn
        .query_row(
            "SELECT checklist_schema, protocol_locked_at, executed_at
             FROM hypotheses WHERE id = ?1",
            [hypothesis_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((schema_json, protocol_locked_at, executed_at)) = hypothesis_state else {
        return Err(format!("Hypothesis not found: {hypothesis_id}"));
    };
    if protocol_locked_at.is_none() {
        return Err("Lock the test protocol before collecting checklist data.".to_string());
    }
    if executed_at.is_some() {
        return Err("Checklist data cannot be changed after test execution.".to_string());
    }
    let schema_json = schema_json
        .ok_or_else(|| format!("Hypothesis '{hypothesis_id}' does not have a checklist schema."))?;
    validate_checklist_entry(&schema_json, data_json)?;
    conn.execute(
        "INSERT INTO hypothesis_data_entries (id, hypothesis_id, data_json, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![id, hypothesis_id, data_json, created_at],
    )
    .map_err(|error| error.to_string())?;
    Ok(DataEntry {
        id: id.to_string(),
        hypothesis_id: hypothesis_id.to_string(),
        data_json: data_json.to_string(),
        created_at,
    })
}

#[tauri::command]
pub fn add_data_entry(
    state: State<AppState>,
    hypothesis_id: String,
    data_json: String,
) -> std::result::Result<DataEntry, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|error| error.to_string())?;
    let id = format!("hypothesis_data_{}", uuid::Uuid::now_v7());
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    add_data_entry_in_db(&conn, &id, &hypothesis_id, &data_json, now)
}

fn list_data_entries(
    conn: &Connection,
    hypothesis_id: &str,
) -> std::result::Result<Vec<DataEntry>, String> {
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
            "SELECT id, hypothesis_id, data_json, created_at
             FROM hypothesis_data_entries
             WHERE hypothesis_id = ?1 ORDER BY created_at, id",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([hypothesis_id], |row| {
            Ok(DataEntry {
                id: row.get(0)?,
                hypothesis_id: row.get(1)?,
                data_json: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_data_entries(
    state: State<AppState>,
    hypothesis_id: String,
) -> std::result::Result<Vec<DataEntry>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    list_data_entries(&conn, &hypothesis_id)
}

fn delete_data_entry_in_db(conn: &Connection, id: &str) -> std::result::Result<(), String> {
    let executed_at: Option<i64> = conn
        .query_row(
            "SELECT hypotheses.executed_at
             FROM hypothesis_data_entries
             JOIN hypotheses ON hypotheses.id = hypothesis_data_entries.hypothesis_id
             WHERE hypothesis_data_entries.id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .flatten();
    if executed_at.is_some() {
        return Err("Checklist data cannot be changed after test execution.".to_string());
    }
    let changed = conn
        .execute("DELETE FROM hypothesis_data_entries WHERE id = ?1", [id])
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err(format!("Hypothesis data entry not found: {id}"));
    }
    Ok(())
}

#[tauri::command]
pub fn delete_data_entry(state: State<AppState>, id: String) -> std::result::Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    delete_data_entry_in_db(&conn, &id)
}

#[tauri::command]
pub fn link_snapshot_to_hypothesis(
    state: State<AppState>,
    hypothesis_id: String,
    snapshot_id: String,
) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|error| error.to_string())?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    link_snapshot_to_hypothesis_in_db(&conn, &hypothesis_id, &snapshot_id, now)
}

fn link_snapshot_to_hypothesis_in_db(
    conn: &Connection,
    hypothesis_id: &str,
    snapshot_id: &str,
    now: i64,
) -> Result<(), String> {
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
        "INSERT OR IGNORE INTO hypothesis_snapshots (hypothesis_id, snapshot_id, created_at)
         VALUES (?1, ?2, ?3)",
        params![hypothesis_id, snapshot_id, now],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn unlink_snapshot_from_hypothesis(
    state: State<AppState>,
    hypothesis_id: String,
    snapshot_id: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    unlink_snapshot_from_hypothesis_in_db(&conn, &hypothesis_id, &snapshot_id)
}

fn unlink_snapshot_from_hypothesis_in_db(
    conn: &Connection,
    hypothesis_id: &str,
    snapshot_id: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM hypothesis_snapshots WHERE hypothesis_id = ?1 AND snapshot_id = ?2",
        params![hypothesis_id, snapshot_id],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_hypothesis_snapshots(
    state: State<AppState>,
    hypothesis_id: String,
) -> Result<Vec<String>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM hypotheses WHERE id = ?1)",
            [&hypothesis_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !exists {
        return Err(format!("Hypothesis not found: {hypothesis_id}"));
    }
    list_hypothesis_snapshots(&conn, &hypothesis_id).map_err(|error| error.to_string())
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
             SET protocol_type = 'text', protocol_steps = ?1,
                 protocol_locked_at = ?2, updated_at = ?2
             WHERE id = ?3 AND protocol_locked_at IS NULL AND executed_at IS NULL",
            params![protocol_steps, now, id],
        )
        .map_err(|error| error.to_string())?;
    let hypothesis = query_hypothesis(&transaction, id).map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(hypothesis)
}

fn validate_protocol_flowchart(flowchart_json: &str) -> std::result::Result<(), String> {
    let flowchart: serde_json::Value = serde_json::from_str(flowchart_json)
        .map_err(|error| format!("Flowchart must be valid JSON: {error}"))?;
    let nodes = flowchart
        .get("nodes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "Flowchart must contain a nodes array.".to_string())?;
    let edges = flowchart
        .get("edges")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "Flowchart must contain an edges array.".to_string())?;
    if nodes.is_empty() {
        return Err("Add at least one node to the flowchart.".to_string());
    }

    let mut node_ids = HashSet::new();
    for node in nodes {
        let id = node
            .get("id")
            .and_then(serde_json::Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| "Every flowchart node must have an ID.".to_string())?;
        if !node_ids.insert(id) {
            return Err(format!("Flowchart contains duplicate node ID: {id}"));
        }
        let kind = node
            .get("data")
            .and_then(|data| data.get("kind"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("Flowchart node '{id}' must have a kind."))?;
        if !matches!(kind, "step" | "decision" | "start-end") {
            return Err(format!("Flowchart node '{id}' has an unsupported kind."));
        }
        if node
            .get("data")
            .and_then(|data| data.get("label"))
            .and_then(serde_json::Value::as_str)
            .filter(|label| !label.trim().is_empty())
            .is_none()
        {
            return Err(format!("Flowchart node '{id}' must have a label."));
        }
        let has_position = node
            .get("position")
            .and_then(serde_json::Value::as_object)
            .is_some_and(|position| {
                position
                    .get("x")
                    .and_then(serde_json::Value::as_f64)
                    .is_some_and(f64::is_finite)
                    && position
                        .get("y")
                        .and_then(serde_json::Value::as_f64)
                        .is_some_and(f64::is_finite)
            });
        if !has_position {
            return Err(format!("Flowchart node '{id}' must have a valid position."));
        }
    }
    let mut edge_ids = HashSet::new();
    for edge in edges {
        let edge_id = edge
            .get("id")
            .and_then(serde_json::Value::as_str)
            .filter(|edge_id| !edge_id.trim().is_empty())
            .ok_or_else(|| "Every flowchart connection must have an ID.".to_string())?;
        if !edge_ids.insert(edge_id) {
            return Err(format!(
                "Flowchart contains duplicate connection ID: {edge_id}"
            ));
        }
        let source = edge
            .get("source")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "Every flowchart edge must have a source node.".to_string())?;
        let target = edge
            .get("target")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "Every flowchart edge must have a target node.".to_string())?;
        if !node_ids.contains(source) || !node_ids.contains(target) {
            return Err("Flowchart edges must connect existing nodes.".to_string());
        }
    }
    Ok(())
}

fn save_protocol_flowchart_in_db(
    conn: &mut Connection,
    id: &str,
    flowchart_json: &str,
    now: i64,
) -> std::result::Result<Hypothesis, String> {
    validate_protocol_flowchart(flowchart_json)?;
    let transaction = conn.transaction().map_err(|error| error.to_string())?;
    let existing: Option<(String, Option<String>, Option<i64>, Option<i64>)> = transaction
        .query_row(
            "SELECT protocol_type, protocol_flowchart_json, protocol_locked_at, executed_at
             FROM hypotheses WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let (protocol_type, existing_flowchart, locked_at, executed_at) =
        existing.ok_or_else(|| format!("Hypothesis not found: {id}"))?;

    if let Some(existing_flowchart) = existing_flowchart {
        if protocol_type == "flowchart" && existing_flowchart == flowchart_json {
            return query_hypothesis(&transaction, id).map_err(|error| error.to_string());
        }
        return Err(if executed_at.is_some() {
            "Protocol cannot be changed after test execution.".to_string()
        } else {
            "Protocol is locked and cannot be changed.".to_string()
        });
    }
    if locked_at.is_some() || executed_at.is_some() {
        return Err("Protocol is locked and cannot be changed.".to_string());
    }

    transaction
        .execute(
            "UPDATE hypotheses
             SET protocol_type = 'flowchart', protocol_flowchart_json = ?1,
                 protocol_completed_steps = '[]', protocol_locked_at = ?2, updated_at = ?2
             WHERE id = ?3 AND protocol_locked_at IS NULL AND executed_at IS NULL",
            params![flowchart_json, now, id],
        )
        .map_err(|error| error.to_string())?;
    let hypothesis = query_hypothesis(&transaction, id).map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(hypothesis)
}

#[tauri::command]
pub fn save_protocol_flowchart(
    state: State<AppState>,
    id: String,
    flowchart_json: String,
) -> std::result::Result<Hypothesis, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|error| error.to_string())?;
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    save_protocol_flowchart_in_db(&mut conn, &id, &flowchart_json, now)
}

fn update_protocol_completion_in_db(
    conn: &Connection,
    id: &str,
    completed_steps_json: &str,
    now: i64,
) -> std::result::Result<Hypothesis, String> {
    let completed_steps: serde_json::Value = serde_json::from_str(completed_steps_json)
        .map_err(|error| format!("Completed steps must be valid JSON: {error}"))?;
    let completed_steps = completed_steps
        .as_array()
        .ok_or_else(|| "Completed steps must be a JSON array.".to_string())?;
    let graph_json: Option<(String, Option<String>, Option<i64>)> = conn
        .query_row(
            "SELECT protocol_type, protocol_flowchart_json, protocol_locked_at
             FROM hypotheses WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let (protocol_type, flowchart_json, locked_at) =
        graph_json.ok_or_else(|| format!("Hypothesis not found: {id}"))?;
    if protocol_type != "flowchart" || locked_at.is_none() {
        return Err("Lock a flowchart protocol before updating its progress.".to_string());
    }
    let flowchart_json = flowchart_json
        .ok_or_else(|| "The locked flowchart protocol is missing its graph.".to_string())?;
    validate_protocol_flowchart(&flowchart_json)?;
    let flowchart: serde_json::Value =
        serde_json::from_str(&flowchart_json).map_err(|error| error.to_string())?;
    let nodes = flowchart
        .get("nodes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "The saved flowchart has no nodes.".to_string())?;
    let node_ids: HashSet<&str> = nodes
        .iter()
        .filter_map(|node| node.get("id").and_then(serde_json::Value::as_str))
        .collect();
    let mut unique_steps = HashSet::new();
    for step in completed_steps {
        let step_id = step
            .as_str()
            .filter(|step_id| !step_id.trim().is_empty())
            .ok_or_else(|| "Completed step IDs must be non-empty strings.".to_string())?;
        if !node_ids.contains(step_id) {
            return Err(format!(
                "Completed step '{step_id}' is not in the saved protocol."
            ));
        }
        if !unique_steps.insert(step_id) {
            return Err(format!("Completed steps contain duplicate ID: {step_id}"));
        }
    }
    conn.execute(
        "UPDATE hypotheses SET protocol_completed_steps = ?1, updated_at = ?2
         WHERE id = ?3 AND protocol_locked_at IS NOT NULL AND protocol_type = 'flowchart'",
        params![completed_steps_json, now, id],
    )
    .map_err(|error| error.to_string())?;
    query_hypothesis(conn, id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_protocol_completion(
    state: State<AppState>,
    id: String,
    completed_steps_json: String,
) -> std::result::Result<Hypothesis, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    let now = i64::try_from(now).map_err(|error| error.to_string())?;
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    update_protocol_completion_in_db(&conn, &id, &completed_steps_json, now)
}

fn build_hypothesis_analysis_prompt(
    conn: &Connection,
    hypothesis_id: &str,
) -> std::result::Result<String, String> {
    let hypothesis = query_hypothesis(conn, hypothesis_id)
        .map_err(|error| format!("Hypothesis not found: {error}"))?;
    let entries = list_data_entries(conn, hypothesis_id)?;
    let protocol = if hypothesis.protocol_type == "flowchart" {
        hypothesis
            .protocol_flowchart_json
            .as_deref()
            .unwrap_or("No flowchart protocol has been saved.")
    } else {
        hypothesis
            .protocol_steps
            .as_deref()
            .unwrap_or("No text protocol has been saved.")
    };
    let entries_json = serde_json::to_string_pretty(&entries)
        .map_err(|error| format!("Could not format checklist entries: {error}"))?;
    let completed_steps = hypothesis
        .protocol_completed_steps
        .as_deref()
        .unwrap_or("[]");
    let checklist_schema = hypothesis
        .checklist_schema
        .as_deref()
        .unwrap_or("No checklist schema defined.");

    Ok(format!(
        "# Hypothesis Lab Data Analysis\n\n\
         Act as a senior data scientist. Analyze this structured research data. \
         Find hidden patterns, calculate win rates, identify boundary conditions, \
         and suggest the next steps. Be explicit about limitations and do not infer \
         unsupported conclusions.\n\n\
         ## Hypothesis (H1)\n{}\n\n\
         ## Null hypothesis (H0)\n{}\n\n\
         ## Research design\n\
         - Independent variable: {}\n\
         - Dependent variable: {}\n\
         - Metric: {}\n\
         - Success threshold: {}\n\n\
         ## Protocol ({})\n{}\n\n\
         ## Completed protocol node IDs\n{}\n\n\
         ## Checklist schema\n{}\n\n\
         ## All collected data entries ({} total)\n```json\n{}\n```\n",
        hypothesis.statement_h1,
        hypothesis.statement_h0,
        hypothesis.independent_var,
        hypothesis.dependent_var,
        hypothesis.metric,
        hypothesis.success_threshold,
        hypothesis.protocol_type,
        protocol,
        completed_steps,
        checklist_schema,
        entries.len(),
        entries_json
    ))
}

#[tauri::command]
pub fn analyze_with_ai(
    state: State<AppState>,
    hypothesis_id: String,
) -> std::result::Result<String, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    build_hypothesis_analysis_prompt(&conn, &hypothesis_id)
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
               AND (
                 (protocol_type = 'text' AND protocol_steps IS NOT NULL
                   AND length(trim(protocol_steps)) > 0)
                 OR
                 (protocol_type = 'flowchart' AND protocol_flowchart_json IS NOT NULL)
               )
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
        return Err(
            "Boundary conditions are required for a partially supported verdict.".to_string(),
        );
    }
    let stored_raw_data: Option<String> = conn
        .query_row(
            "SELECT raw_data FROM hypotheses WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
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
            params![
                evidence_notes,
                verdict,
                data_analysis,
                boundary_conditions,
                now,
                id
            ],
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
    hypothesis.linked_snapshots = list_hypothesis_snapshots(conn, id)?;
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
        linked_snapshots: Vec::new(),
        protocol_steps: row.get(16)?,
        protocol_locked_at: row.get(17)?,
        raw_data: row.get(18)?,
        data_analysis: row.get(19)?,
        executed_at: row.get(20)?,
        boundary_conditions: row.get(21)?,
        refined_from_id: row.get(22)?,
        checklist_schema: row.get(23)?,
        protocol_type: row.get(24)?,
        protocol_flowchart_json: row.get(25)?,
        protocol_completed_steps: row.get(26)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        add_data_entry_in_db, add_hypothesis_insight_in_db, build_hypothesis_analysis_prompt,
        create_child_hypothesis_in_db, delete_data_entry_in_db, delete_hypothesis_insight_in_db,
        graduate_to_library_in_db, init_schema, link_snapshot_to_hypothesis_in_db,
        list_data_entries, list_hypotheses, list_hypothesis_insights, list_hypothesis_snapshots,
        promote_inquiry, record_hypothesis_execution_in_db, record_verdict_in_db,
        save_checklist_schema_in_db, save_hypothesis_protocol_in_db, save_protocol_flowchart_in_db,
        unlink_snapshot_from_hypothesis_in_db, update_protocol_completion_in_db, LinkedDefinition,
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
    fn checklist_schema_and_data_entries_are_validated_and_managed() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        promote(&mut connection, "inquiry-1", "hypothesis-1", 10).expect("promote inquiry");
        let schema_json = serde_json::json!([
            { "label": "Trend", "type": "select", "options": ["Bullish", "Bearish"] },
            { "label": "Score", "type": "number", "options": [] },
            { "label": "Confirmed", "type": "boolean", "options": [] },
            { "label": "Notes", "type": "text", "options": [] }
        ])
        .to_string();

        assert!(save_checklist_schema_in_db(&connection, "missing", &schema_json, 15).is_err());
        assert!(
            save_checklist_schema_in_db(&connection, "hypothesis-1", &schema_json, 15).is_err()
        );
        save_hypothesis_protocol_in_db(
            &mut connection,
            "hypothesis-1",
            "Collect observations.",
            14,
        )
        .expect("lock test protocol");
        assert!(save_checklist_schema_in_db(&connection, "hypothesis-1", "{}", 15).is_err());
        assert!(save_checklist_schema_in_db(
            &connection,
            "hypothesis-1",
            r#"[{"label":"Trend","type":"select","options":[]}]"#,
            15
        )
        .is_err());

        let hypothesis = save_checklist_schema_in_db(&connection, "hypothesis-1", &schema_json, 15)
            .expect("save checklist schema");
        assert_eq!(
            hypothesis.checklist_schema.as_deref(),
            Some(schema_json.as_str())
        );
        assert_eq!(hypothesis.updated_at, 15);

        let row_json = serde_json::json!({
            "Trend": "Bullish",
            "Score": 4.5,
            "Confirmed": true,
            "Notes": "Breakout held"
        })
        .to_string();
        assert!(add_data_entry_in_db(
            &connection,
            "invalid-entry",
            "hypothesis-1",
            r#"{"Trend":"Sideways","Score":"high","Confirmed":true,"Notes":""}"#,
            20,
        )
        .is_err());
        let first = add_data_entry_in_db(&connection, "entry-1", "hypothesis-1", &row_json, 20)
            .expect("add valid checklist entry");
        let second = add_data_entry_in_db(&connection, "entry-2", "hypothesis-1", &row_json, 21)
            .expect("add second checklist entry");
        assert_eq!(first.hypothesis_id, "hypothesis-1");
        assert_eq!(first.data_json, row_json);
        assert_eq!(
            list_data_entries(&connection, "hypothesis-1").expect("list checklist entries"),
            vec![first, second]
        );
        assert!(
            save_checklist_schema_in_db(&connection, "hypothesis-1", &schema_json, 22).is_err()
        );

        delete_data_entry_in_db(&connection, "entry-1").expect("delete checklist entry");
        assert_eq!(
            list_data_entries(&connection, "hypothesis-1")
                .expect("list remaining entries")
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-2"]
        );
        assert!(delete_data_entry_in_db(&connection, "missing").is_err());
        assert!(list_data_entries(&connection, "missing").is_err());
        record_hypothesis_execution_in_db(&mut connection, "hypothesis-1", "Test complete", 30)
            .expect("record test execution");
        assert!(add_data_entry_in_db(
            &connection,
            "entry-after-execution",
            "hypothesis-1",
            &row_json,
            31,
        )
        .is_err());
        assert!(delete_data_entry_in_db(&connection, "entry-2").is_err());
        assert!(
            save_checklist_schema_in_db(&connection, "hypothesis-1", &schema_json, 32).is_err()
        );
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
            None,
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

        assert!(save_hypothesis_protocol_in_db(
            &mut connection,
            "hypothesis-1",
            "Changed protocol",
            40,
        )
        .is_err());
    }

    #[test]
    fn flowchart_protocol_completion_and_analysis_prompt_are_managed() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        promote(&mut connection, "inquiry-1", "hypothesis-1", 10).expect("promote inquiry");
        let graph_json = serde_json::json!({
            "nodes": [
                {
                    "id": "start",
                    "type": "protocol",
                    "position": { "x": 0, "y": 0 },
                    "data": { "label": "Start test", "kind": "start-end" }
                },
                {
                    "id": "observe",
                    "type": "protocol",
                    "position": { "x": 0, "y": 100 },
                    "data": { "label": "Record observation", "kind": "step" }
                }
            ],
            "edges": [{ "id": "edge-1", "source": "start", "target": "observe" }]
        })
        .to_string();

        assert!(save_protocol_flowchart_in_db(
            &mut connection,
            "hypothesis-1",
            r#"{"nodes":[],"edges":[]}"#,
            15
        )
        .is_err());
        assert!(save_protocol_flowchart_in_db(
            &mut connection,
            "hypothesis-1",
            r#"{"nodes":[{"id":"one","data":{"label":"Start","kind":"start-end"}}],"edges":[{"id":"bad","source":"one","target":"missing"}]}"#,
            15
        )
        .is_err());

        let saved = save_protocol_flowchart_in_db(&mut connection, "hypothesis-1", &graph_json, 20)
            .expect("save flowchart protocol");
        assert_eq!(saved.protocol_type, "flowchart");
        assert_eq!(
            saved.protocol_flowchart_json.as_deref(),
            Some(graph_json.as_str())
        );
        assert_eq!(saved.protocol_completed_steps.as_deref(), Some("[]"));
        assert_eq!(saved.protocol_locked_at, Some(20));

        let completed =
            update_protocol_completion_in_db(&connection, "hypothesis-1", r#"["start"]"#, 25)
                .expect("update protocol progress");
        assert_eq!(
            completed.protocol_completed_steps.as_deref(),
            Some(r#"["start"]"#)
        );
        assert!(update_protocol_completion_in_db(
            &connection,
            "hypothesis-1",
            r#"["not-a-node"]"#,
            26
        )
        .is_err());
        assert!(update_protocol_completion_in_db(
            &connection,
            "hypothesis-1",
            r#"["start","start"]"#,
            27
        )
        .is_err());

        let checklist_schema =
            r#"[{"label":"Trend","type":"select","options":["Bullish","Bearish"]}]"#;
        save_checklist_schema_in_db(&connection, "hypothesis-1", checklist_schema, 28)
            .expect("save checklist schema");
        add_data_entry_in_db(
            &connection,
            "entry-1",
            "hypothesis-1",
            r#"{"Trend":"Bullish"}"#,
            29,
        )
        .expect("add data entry");
        let prompt = build_hypothesis_analysis_prompt(&connection, "hypothesis-1")
            .expect("build analysis prompt");
        assert!(prompt.contains("If X then Y"));
        assert!(prompt.contains("X does not affect Y"));
        assert!(prompt.contains("Record observation"));
        assert!(prompt.contains("Bullish"));
        assert!(prompt.contains("1 total"));

        record_hypothesis_execution_in_db(&mut connection, "hypothesis-1", "Raw test data", 30)
            .expect("record flowchart protocol execution");
        assert!(
            save_protocol_flowchart_in_db(&mut connection, "hypothesis-1", &graph_json, 31).is_ok()
        );
        assert!(save_protocol_flowchart_in_db(
            &mut connection,
            "hypothesis-1",
            r#"{"nodes":[{"id":"changed","data":{"label":"Changed","kind":"step"}}],"edges":[]}"#,
            32
        )
        .is_err());
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

        let hypothesis = graduate_to_library_in_db(&mut connection, "hypothesis-1", None, 30)
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
        let second =
            graduate_to_library_in_db(&mut connection, "hypothesis-2", Some(&group_id), 40)
                .expect("append second hypothesis");

        assert_eq!(
            second.promoted_to_library_id.as_deref(),
            Some(group_id.as_str())
        );
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
            .query_row("SELECT COUNT(*) FROM library_item_groups", [], |row| {
                row.get(0)
            })
            .expect("count Library groups");
        assert_eq!(groups, 0);
    }

    #[test]
    fn child_hypothesis_inherits_inquiry_and_records_parent() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        let definition = insert_test_definition(&connection);
        promote_with_definitions(
            &mut connection,
            "inquiry-1",
            "hypothesis-1",
            std::slice::from_ref(&definition),
            10,
        )
        .expect("promote inquiry");
        save_hypothesis_protocol_in_db(&mut connection, "hypothesis-1", "Run the test.", 15)
            .expect("save protocol");
        record_hypothesis_execution_in_db(&mut connection, "hypothesis-1", "Raw observations", 18)
            .expect("record execution");
        let parent = record_verdict_in_db(
            &connection,
            "hypothesis-1",
            "The result held under measured conditions.",
            "supported",
            Some("Raw observations"),
            Some("Evidence supports H1."),
            Some("The measured cohort."),
            20,
        )
        .expect("record supported verdict");

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
        assert_eq!(child.refined_from_id.as_deref(), Some(parent.id.as_str()));
        assert_eq!(child.created_at, 20);
        assert_eq!(child.linked_definitions, vec![definition]);
    }

    #[test]
    fn child_hypothesis_can_refine_an_inconclusive_verdict() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        promote(&mut connection, "inquiry-1", "hypothesis-1", 10).expect("promote inquiry");
        save_hypothesis_protocol_in_db(&mut connection, "hypothesis-1", "Run the test.", 15)
            .expect("save protocol");
        record_hypothesis_execution_in_db(&mut connection, "hypothesis-1", "Raw observations", 18)
            .expect("record execution");
        record_verdict_in_db(
            &connection,
            "hypothesis-1",
            "The result was inconclusive.",
            "inconclusive",
            Some("Raw observations"),
            Some("The sample was too small."),
            None,
            20,
        )
        .expect("record inconclusive verdict");

        let child = create_child_hypothesis_in_db(
            &mut connection,
            "hypothesis-1",
            "hypothesis-child",
            "If X in a larger sample then Y",
            "X does not affect Y",
            "X in a larger sample",
            "Y",
            "Y per day",
            "Y increases by 10%",
            &[],
            25,
        )
        .expect("create child from inconclusive hypothesis");

        assert_eq!(child.parent_hypothesis_id.as_deref(), Some("hypothesis-1"));
        assert_eq!(child.refined_from_id.as_deref(), Some("hypothesis-1"));
    }

    #[test]
    fn hypothesis_snapshot_links_are_idempotent_and_included_in_reads() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        promote(&mut connection, "inquiry-1", "hypothesis-1", 10).expect("promote inquiry");

        link_snapshot_to_hypothesis_in_db(&connection, "hypothesis-1", "snapshot-1", 20)
            .expect("link snapshot");
        link_snapshot_to_hypothesis_in_db(&connection, "hypothesis-1", "snapshot-1", 30)
            .expect("duplicate link is ignored");
        link_snapshot_to_hypothesis_in_db(&connection, "hypothesis-1", "snapshot-2", 40)
            .expect("link second snapshot");

        let hypothesis = list_hypotheses(&connection)
            .expect("list hypotheses")
            .into_iter()
            .next()
            .expect("hypothesis");
        assert_eq!(
            hypothesis.linked_snapshots,
            vec!["snapshot-1".to_string(), "snapshot-2".to_string()]
        );
        assert_eq!(
            list_hypothesis_snapshots(&connection, "hypothesis-1").expect("list snapshots"),
            hypothesis.linked_snapshots
        );

        unlink_snapshot_from_hypothesis_in_db(&connection, "hypothesis-1", "snapshot-1")
            .expect("unlink snapshot");
        assert_eq!(
            list_hypothesis_snapshots(&connection, "hypothesis-1").expect("list after unlink"),
            vec!["snapshot-2".to_string()]
        );
    }

    #[test]
    fn partially_supported_verdict_records_boundaries_and_graduates_with_them() {
        let mut connection = setup();
        select_inquiry(&mut connection, "inquiry-1");
        promote(&mut connection, "inquiry-1", "hypothesis-1", 10).expect("promote inquiry");
        save_hypothesis_protocol_in_db(&mut connection, "hypothesis-1", "Run the test.", 15)
            .expect("save protocol");
        record_hypothesis_execution_in_db(&mut connection, "hypothesis-1", "Raw observations", 18)
            .expect("record execution");

        assert!(record_verdict_in_db(
            &connection,
            "hypothesis-1",
            "The result was mixed.",
            "partially_supported",
            Some("Raw observations"),
            Some("The result held in trending markets."),
            None,
            19,
        )
        .is_err());
        let hypothesis = record_verdict_in_db(
            &connection,
            "hypothesis-1",
            "The result was mixed.",
            "partially_supported",
            Some("Raw observations"),
            Some("The result held in trending markets."),
            Some("Trending markets"),
            20,
        )
        .expect("record partial verdict with boundary conditions");
        assert_eq!(hypothesis.verdict.as_deref(), Some("partially_supported"));
        assert_eq!(
            hypothesis.boundary_conditions.as_deref(),
            Some("Trending markets")
        );
        let child = create_child_hypothesis_in_db(
            &mut connection,
            "hypothesis-1",
            "hypothesis-child",
            "If X in a trending market then Y",
            "X does not affect Y",
            "X in a trending market",
            "Y",
            "Y per day",
            "Y increases by 10%",
            &[],
            25,
        )
        .expect("create refinement from partial hypothesis");
        assert_eq!(child.refined_from_id.as_deref(), Some("hypothesis-1"));

        let graduated = graduate_to_library_in_db(&mut connection, "hypothesis-1", None, 30)
            .expect("graduate partial hypothesis");
        let group_id = graduated.promoted_to_library_id.expect("Library group ID");
        let (name, snapshot_json): (String, String) = connection
            .query_row(
                "SELECT name, snapshot_json FROM library_item_groups WHERE id = ?1",
                [&group_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read Library group");
        assert_eq!(name, "If X then Y (Valid only when: Trending markets)");
        let snapshot: serde_json::Value =
            serde_json::from_str(&snapshot_json).expect("parse Library snapshot");
        assert!(snapshot["nodes"][0]["title"]
            .as_str()
            .expect("Library node title")
            .contains("Valid only when: Trending markets"));
        assert!(snapshot["nodes"][0]["content"]
            .as_str()
            .expect("Library node content")
            .contains("Boundary conditions: Trending markets"));
    }
}
