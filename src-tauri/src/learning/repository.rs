use super::*;
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const LEARNING_SCHEMA: &str = include_str!("schema.sql");
const LEARNING_MIGRATION_ID: &str = "phase6-learning-001";

fn id() -> String {
    Uuid::now_v7().to_string()
}

fn now_us() -> Result<i64> {
    let value = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros();
    i64::try_from(value).map_err(|_| anyhow!("system timestamp exceeds the supported range"))
}

fn required(value: &str, name: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{name} must not be empty."));
    }
    Ok(())
}

fn valid_unit_score(value: f64, name: &str) -> Result<()> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(anyhow!("{name} must be finite and between 0 and 1."));
    }
    Ok(())
}

fn valid_positive_score(value: f64, name: &str) -> Result<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(anyhow!("{name} must be a positive finite number."));
    }
    Ok(())
}

fn json<T: serde::Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?)
}

fn parse_json<T: for<'de> serde::Deserialize<'de>>(value: &str) -> rusqlite::Result<T> {
    serde_json::from_str(value).map_err(|_| rusqlite::Error::InvalidQuery)
}

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS learning_schema_migrations (
             version INTEGER PRIMARY KEY,
             migration_id TEXT NOT NULL UNIQUE,
             checksum TEXT NOT NULL,
             applied_at_us INTEGER NOT NULL
         );",
    )?;
    let foreign_keys: i64 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
    if foreign_keys != 1 {
        return Err(anyhow!("SQLite foreign-key enforcement is required for Learning writes."));
    }

    conn.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| -> Result<()> {
        let checksum = format!("{:x}", Sha256::digest(LEARNING_SCHEMA.as_bytes()));
        let applied: Option<(String, String)> = conn
            .query_row(
                "SELECT migration_id, checksum FROM learning_schema_migrations WHERE version = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((migration_id, applied_checksum)) = applied {
            if migration_id != LEARNING_MIGRATION_ID || applied_checksum != checksum {
                return Err(anyhow!(
                    "Learning migration 1 differs from the applied migration; add a new numbered migration."
                ));
            }
            return Ok(());
        }

        conn.execute_batch(LEARNING_SCHEMA)?;
        conn.execute(
            "INSERT INTO learning_schema_migrations
                (version, migration_id, checksum, applied_at_us)
             VALUES (1, ?1, ?2, ?3)",
            params![LEARNING_MIGRATION_ID, checksum, now_us()?],
        )?;
        Ok(())
    })();
    match result {
        Ok(()) => conn.execute_batch("COMMIT")?,
        Err(error) => {
            conn.execute_batch("ROLLBACK")?;
            return Err(error);
        }
    }
    Ok(())
}

fn project_context(
    conn: &Connection,
    project_id: &str,
) -> Result<(String, String, String)> {
    conn.query_row(
        "SELECT learner_id, framework_id, framework_version_id
         FROM learning_projects WHERE id = ?1",
        [project_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .map_err(Into::into)
}

pub fn create_learner(
    conn: &mut Connection,
    request: &CreateLearnerReq,
) -> Result<LearningLearnerDto> {
    required(&request.display_name, "Learner display name")?;
    let learner_id = id();
    let created_at_us = now_us()?;
    conn.execute(
        "INSERT INTO learning_learners (id, display_name, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4)",
        params![learner_id, request.display_name.trim(), request.created_by, created_at_us],
    )?;
    Ok(LearningLearnerDto {
        id: learner_id,
        display_name: request.display_name.trim().to_string(),
        created_by: request.created_by.clone(),
        created_at_us,
    })
}

pub fn create_project(
    conn: &mut Connection,
    request: &CreateLearningProjectReq,
) -> Result<LearningProjectDto> {
    required(&request.title, "Learning project title")?;
    let project_id = id();
    let created_at_us = now_us()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute(
        "INSERT INTO learning_projects
            (id, title, learner_id, framework_id, framework_version_id, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            project_id,
            request.title.trim(),
            request.learner_id,
            request.framework_id,
            request.framework_version_id,
            request.created_by,
            created_at_us
        ],
    )?;
    tx.commit()?;
    query_project(conn, &project_id)
}

pub fn create_objective_version(
    conn: &mut Connection,
    request: &CreateLearningObjectiveReq,
) -> Result<LearningObjectiveVersionDto> {
    required(&request.title, "Learning objective title")?;
    required(&request.observable_outcome, "Observable learning outcome")?;
    required(&request.criteria_text, "Learning objective criteria")?;
    required(&request.level, "Learning level")?;
    let (learner_id, framework_id, framework_version_id) =
        project_context(conn, &request.learning_project_id)?;
    let _ = learner_id;
    let created_at_us = now_us()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let objective_id = if let Some(objective_id) = &request.objective_id {
        let objective_project: String = tx.query_row(
            "SELECT learning_project_id FROM learning_objectives WHERE id = ?1",
            [objective_id],
            |row| row.get(0),
        )?;
        if objective_project != request.learning_project_id {
            return Err(anyhow!("Learning objective belongs to a different learning project."));
        }
        objective_id.clone()
    } else {
        let objective_id = id();
        tx.execute(
            "INSERT INTO learning_objectives
                (id, learning_project_id, title, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                objective_id,
                request.learning_project_id,
                request.title.trim(),
                request.created_by,
                created_at_us
            ],
        )?;
        objective_id
    };
    let version_no: i64 = tx.query_row(
        "SELECT COALESCE(MAX(version_no), 0) + 1
         FROM learning_objective_versions WHERE objective_id = ?1",
        [&objective_id],
        |row| row.get(0),
    )?;
    if let Some(parent_id) = &request.parent_version_id {
        let parent_objective: String = tx.query_row(
            "SELECT objective_id FROM learning_objective_versions WHERE id = ?1",
            [parent_id],
            |row| row.get(0),
        )?;
        if parent_objective != objective_id {
            return Err(anyhow!("Objective revisions must parent the same objective identity."));
        }
    } else if version_no > 1 {
        return Err(anyhow!("A revised objective version must pin its parent version."));
    }
    let objective_version_id = id();
    tx.execute(
        "INSERT INTO learning_objective_versions
            (id, objective_id, version_no, learning_project_id, framework_id,
             framework_version_id, parent_version_id, observable_outcome, criteria_text,
             level, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            objective_version_id,
            objective_id,
            version_no,
            request.learning_project_id,
            framework_id,
            framework_version_id,
            request.parent_version_id,
            request.observable_outcome.trim(),
            request.criteria_text.trim(),
            request.level.trim(),
            request.created_by,
            created_at_us
        ],
    )?;
    let mut seen = HashSet::new();
    for prerequisite_id in &request.prerequisite_version_ids {
        if !seen.insert(prerequisite_id) {
            return Err(anyhow!("Objective prerequisite list contains duplicates."));
        }
        if prerequisite_id == &objective_version_id {
            return Err(anyhow!("An objective cannot be its own prerequisite."));
        }
        tx.execute(
            "INSERT INTO learning_objective_prerequisites
                (objective_version_id, prerequisite_version_id, learning_project_id,
                 framework_id, framework_version_id, rationale, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5, 'Declared objective prerequisite.', ?6, ?7)",
            params![
                objective_version_id,
                prerequisite_id,
                request.learning_project_id,
                framework_id,
                framework_version_id,
                request.created_by,
                created_at_us
            ],
        )?;
    }
    tx.commit()?;
    query_objective(conn, &objective_version_id)
}

pub fn create_assessment_version(
    conn: &mut Connection,
    request: &CreateAssessmentVersionReq,
) -> Result<AssessmentVersionDto> {
    required(&request.title, "Assessment title")?;
    required(&request.change_rationale, "Assessment version rationale")?;
    required(&request.rubric_title, "Assessment rubric title")?;
    valid_unit_score(request.rubric_pass_threshold, "Rubric pass threshold")?;
    if request.questions.is_empty() || request.rubric_criteria.is_empty() {
        return Err(anyhow!(
            "An assessment version requires at least one question and one rubric criterion."
        ));
    }
    let (_, framework_id, framework_version_id) = project_context(conn, &request.learning_project_id)?;
    let now = now_us()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let (assessment_id, version_no, rubric_id, rubric_version_no) =
        if let Some(assessment_id) = &request.assessment_id {
            let existing: (String, String) = tx.query_row(
                "SELECT learning_project_id, title FROM assessments WHERE id = ?1",
                [assessment_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            if existing.0 != request.learning_project_id || existing.1 != request.title.trim() {
                return Err(anyhow!(
                    "Assessment versions must retain their identity and learning project."
                ));
            }
            let version_no: i64 = tx.query_row(
                "SELECT COALESCE(MAX(version_no), 0) + 1
                 FROM assessment_versions WHERE assessment_id = ?1",
                [assessment_id],
                |row| row.get(0),
            )?;
            let rubric_id: String = tx.query_row(
                "SELECT rv.rubric_id FROM assessment_versions av
                 JOIN assessment_rubric_versions rv ON rv.id = av.rubric_version_id
                 WHERE av.assessment_id = ?1 ORDER BY av.version_no DESC LIMIT 1",
                [assessment_id],
                |row| row.get(0),
            )?;
            let rubric_version_no: i64 = tx.query_row(
                "SELECT COALESCE(MAX(version_no), 0) + 1
                 FROM assessment_rubric_versions WHERE rubric_id = ?1",
                [&rubric_id],
                |row| row.get(0),
            )?;
            (
                assessment_id.clone(),
                version_no,
                rubric_id,
                rubric_version_no,
            )
        } else {
            let assessment_id = id();
            let rubric_id = id();
            tx.execute(
                "INSERT INTO assessments
                    (id, learning_project_id, title, created_by, created_at_us)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    assessment_id,
                    request.learning_project_id,
                    request.title.trim(),
                    request.created_by,
                    now
                ],
            )?;
            tx.execute(
                "INSERT INTO assessment_rubrics
                    (id, learning_project_id, framework_id, framework_version_id,
                     title, created_by, created_at_us)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    rubric_id,
                    request.learning_project_id,
                    framework_id,
                    framework_version_id,
                    request.rubric_title.trim(),
                    request.created_by,
                    now
                ],
            )?;
            (assessment_id, 1, rubric_id, 1)
        };
    let rubric_version_id = id();
    tx.execute(
        "INSERT INTO assessment_rubric_versions
            (id, rubric_id, version_no, learning_project_id, framework_version_id,
             pass_threshold, status, created_by, created_at_us, published_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'draft', ?7, ?8, NULL)",
        params![
            rubric_version_id,
            rubric_id,
            rubric_version_no,
            request.learning_project_id,
            framework_version_id,
            request.rubric_pass_threshold,
            request.created_by,
            now
        ],
    )?;
    let assessment_version_id = id();
    tx.execute(
        "INSERT INTO assessment_versions
            (id, assessment_id, version_no, learning_project_id, framework_id,
             framework_version_id, rubric_version_id, status, change_rationale,
             created_by, created_at_us, published_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'draft', ?8, ?9, ?10, NULL)",
        params![
            assessment_version_id,
            assessment_id,
            version_no,
            request.learning_project_id,
            framework_id,
            framework_version_id,
            rubric_version_id,
            request.change_rationale.trim(),
            request.created_by,
            now
        ],
    )?;
    let mut objective_versions = HashSet::new();
    let mut question_orders = HashSet::new();
    for question in &request.questions {
        required(&question.prompt, "Assessment question prompt")?;
        if !matches!(
            question.response_kind.as_str(),
            "text" | "number" | "choice" | "json"
        ) {
            return Err(anyhow!("Unsupported assessment response kind."));
        }
        if question.question_order < 0 || !question_orders.insert(question.question_order) {
            return Err(anyhow!("Assessment question order must be unique and non-negative."));
        }
        valid_positive_score(question.max_points, "Question maximum points")?;
        objective_versions.insert(question.objective_version_id.clone());
        let question_id = id();
        tx.execute(
            "INSERT INTO assessment_questions
                (id, assessment_version_id, learning_project_id, framework_id,
                 framework_version_id, objective_version_id, question_order,
                 prompt, response_kind, max_points, required)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                question_id,
                assessment_version_id,
                request.learning_project_id,
                framework_id,
                framework_version_id,
                question.objective_version_id,
                question.question_order,
                question.prompt.trim(),
                question.response_kind,
                question.max_points,
                i64::from(question.required)
            ],
        )?;
    }
    let mut criterion_keys = HashSet::new();
    for criterion in &request.rubric_criteria {
        required(&criterion.criterion_key, "Rubric criterion key")?;
        required(&criterion.description, "Rubric criterion description")?;
        valid_positive_score(criterion.max_points, "Rubric criterion maximum points")?;
        if !criterion_keys.insert(criterion.criterion_key.trim()) {
            return Err(anyhow!("Rubric criterion keys must be unique."));
        }
        objective_versions.insert(criterion.objective_version_id.clone());
        tx.execute(
            "INSERT INTO assessment_rubric_criteria
                (id, rubric_version_id, learning_project_id, framework_id,
                 framework_version_id, objective_version_id, criterion_key,
                 description, max_points)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id(),
                rubric_version_id,
                request.learning_project_id,
                framework_id,
                framework_version_id,
                criterion.objective_version_id,
                criterion.criterion_key.trim(),
                criterion.description.trim(),
                criterion.max_points
            ],
        )?;
    }
    for objective_version_id in objective_versions {
        tx.execute(
            "INSERT INTO assessment_version_objectives
                (assessment_version_id, objective_version_id, learning_project_id,
                 framework_id, framework_version_id)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                assessment_version_id,
                objective_version_id,
                request.learning_project_id,
                framework_id,
                framework_version_id
            ],
        )?;
    }
    tx.commit()?;
    query_assessment_version(conn, &assessment_version_id)
}

pub fn review_assessment(
    conn: &mut Connection,
    request: &ReviewAssessmentReq,
) -> Result<AssessmentVersionDto> {
    required(&request.rationale, "Assessment review rationale")?;
    let now = now_us()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let status: String = tx.query_row(
        "SELECT status FROM assessment_versions WHERE id = ?1",
        [&request.assessment_version_id],
        |row| row.get(0),
    )?;
    if !matches!(status.as_str(), "draft" | "review") {
        return Err(anyhow!("Only draft or review assessments may receive review."));
    }
    tx.execute(
        "INSERT INTO assessment_reviews
            (id, assessment_version_id, review_status, reviewer_id, rationale, reviewed_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id(),
            request.assessment_version_id,
            request.review_status.as_str(),
            request.reviewer_id,
            request.rationale.trim(),
            now
        ],
    )?;
    if status == "draft" {
        tx.execute(
            "UPDATE assessment_versions SET status = 'review' WHERE id = ?1",
            [&request.assessment_version_id],
        )?;
    }
    tx.commit()?;
    query_assessment_version(conn, &request.assessment_version_id)
}

pub fn transition_assessment_version(
    conn: &mut Connection,
    request: &TransitionAssessmentVersionReq,
) -> Result<AssessmentVersionDto> {
    let now = now_us()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current: String = tx.query_row(
        "SELECT status FROM assessment_versions WHERE id = ?1",
        [&request.assessment_version_id],
        |row| row.get(0),
    )?;
    let next = request.next_status.as_str();
    let allowed = matches!(
        (current.as_str(), next),
        ("review", "published") | ("published", "deprecated")
    );
    if !allowed {
        return Err(anyhow!("Invalid assessment-version lifecycle transition."));
    }
    if next == "published" {
        let (rubric_id, rubric_status): (String, String) = tx.query_row(
            "SELECT rubric_version_id, rv.status FROM assessment_versions av
             JOIN assessment_rubric_versions rv ON rv.id = av.rubric_version_id
             WHERE av.id = ?1",
            [&request.assessment_version_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if rubric_status == "draft" || rubric_status == "review" {
            tx.execute(
                "UPDATE assessment_rubric_versions
                 SET status = 'published', published_at_us = ?1 WHERE id = ?2",
                params![now, rubric_id],
            )?;
        } else if rubric_status != "published" {
            return Err(anyhow!("Assessment rubric is deprecated and cannot be published."));
        }
        tx.execute(
            "UPDATE assessment_versions
             SET status = 'published', published_at_us = ?1 WHERE id = ?2",
            params![now, request.assessment_version_id],
        )?;
    } else {
        tx.execute(
            "UPDATE assessment_versions SET status = 'deprecated' WHERE id = ?1",
            [&request.assessment_version_id],
        )?;
    }
    tx.commit()?;
    query_assessment_version(conn, &request.assessment_version_id)
}

pub fn create_attempt(
    conn: &mut Connection,
    request: &CreateAssessmentAttemptReq,
) -> Result<AssessmentAttemptDto> {
    let (framework_id, framework_version_id, status): (String, String, String) =
        conn.query_row(
            "SELECT framework_id, framework_version_id, status
             FROM assessment_versions WHERE id = ?1 AND learning_project_id = ?2",
            params![request.assessment_version_id, request.learning_project_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    if status != "published" {
        return Err(anyhow!("Only a published exact assessment version can be attempted."));
    }
    let (project_learner, project_framework, project_framework_version) =
        project_context(conn, &request.learning_project_id)?;
    if request.learner_id != project_learner
        || framework_id != project_framework
        || framework_version_id != project_framework_version
    {
        return Err(anyhow!("Attempt learner or framework context does not match the project."));
    }
    let attempt_id = id();
    conn.execute(
        "INSERT INTO assessment_attempts
            (id, learning_project_id, learner_id, assessment_version_id, framework_id,
             framework_version_id, status, started_at_us, submitted_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'in_progress', ?7, NULL)",
        params![
            attempt_id,
            request.learning_project_id,
            request.learner_id,
            request.assessment_version_id,
            framework_id,
            framework_version_id,
            now_us()?
        ],
    )?;
    query_attempt(conn, &attempt_id)
}

pub fn save_attempt_response(
    conn: &Connection,
    request: &SaveAttemptResponseReq,
) -> Result<AttemptResponseDto> {
    let now = now_us()?;
    let response_json = json(&request.response)?;
    conn.execute(
        "INSERT INTO attempt_responses
            (id, attempt_id, assessment_question_id, response_json, created_at_us, updated_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)
         ON CONFLICT(attempt_id, assessment_question_id)
         DO UPDATE SET response_json = excluded.response_json, updated_at_us = excluded.updated_at_us",
        params![
            id(),
            request.attempt_id,
            request.assessment_question_id,
            response_json,
            now
        ],
    )?;
    conn.query_row(
        "SELECT id, assessment_question_id, response_json, updated_at_us
         FROM attempt_responses WHERE attempt_id = ?1 AND assessment_question_id = ?2",
        params![request.attempt_id, request.assessment_question_id],
        |row| {
            Ok(AttemptResponseDto {
                id: row.get(0)?,
                assessment_question_id: row.get(1)?,
                response: parse_json(&row.get::<_, String>(2)?)?,
                updated_at_us: row.get(3)?,
            })
        },
    )
    .map_err(Into::into)
}

pub fn submit_attempt(conn: &mut Connection, attempt_id: &str) -> Result<AssessmentAttemptDto> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let status: String = tx.query_row(
        "SELECT status FROM assessment_attempts WHERE id = ?1",
        [attempt_id],
        |row| row.get(0),
    )?;
    if status != "in_progress" {
        return Err(anyhow!("Only an in-progress assessment attempt can be submitted."));
    }
    let missing_required: bool = tx.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM assessment_questions q
            JOIN assessment_attempts a ON a.assessment_version_id = q.assessment_version_id
            WHERE a.id = ?1 AND q.required = 1
              AND NOT EXISTS (
                  SELECT 1 FROM attempt_responses r
                  WHERE r.attempt_id = a.id AND r.assessment_question_id = q.id
              )
        )",
        [attempt_id],
        |row| row.get(0),
    )?;
    if missing_required {
        return Err(anyhow!("Every required assessment question needs a response before submission."));
    }
    tx.execute(
        "UPDATE assessment_attempts
         SET status = 'submitted', submitted_at_us = ?1 WHERE id = ?2",
        params![now_us()?, attempt_id],
    )?;
    tx.commit()?;
    query_attempt(conn, attempt_id)
}

pub fn evaluate_attempt(
    conn: &mut Connection,
    request: &EvaluateAssessmentAttemptReq,
) -> Result<AttemptEvaluationDto> {
    required(&request.evaluator_id, "Evaluator identity")?;
    required(&request.rationale, "Evaluation rationale")?;
    required(&request.feedback, "Evaluator feedback")?;
    let mut criterion_ids = HashSet::new();
    for score in &request.criterion_scores {
        if !score.score.is_finite() || score.score < 0.0 {
            return Err(anyhow!("Criterion scores must be finite and non-negative."));
        }
        if !criterion_ids.insert(&score.rubric_criterion_id) {
            return Err(anyhow!("Rubric criterion scores must not be duplicated."));
        }
    }
    let now = now_us()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let (rubric_version_id, pass_threshold, learner_id): (String, f64, String) = tx.query_row(
        "SELECT av.rubric_version_id, rv.pass_threshold, a.learner_id
         FROM assessment_attempts a
         JOIN assessment_versions av ON av.id = a.assessment_version_id
         JOIN assessment_rubric_versions rv ON rv.id = av.rubric_version_id
         WHERE a.id = ?1 AND a.status = 'submitted'",
        [&request.attempt_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    if request.evaluator_id.trim() == learner_id {
        return Err(anyhow!("An assessment learner cannot evaluate their own attempt."));
    }
    let expected: HashMap<String, f64> = {
        let mut statement = tx.prepare(
            "SELECT id, max_points FROM assessment_rubric_criteria
             WHERE rubric_version_id = ?1",
        )?;
        let rows = statement
            .query_map([&rubric_version_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<HashMap<_, _>>>()?;
        rows
    };
    if expected.is_empty() || expected.len() != request.criterion_scores.len() {
        return Err(anyhow!("Every criterion in the pinned rubric must be evaluated exactly once."));
    }
    let mut total_score = 0.0;
    let mut total_max = 0.0;
    for score in &request.criterion_scores {
        let max_points = expected
            .get(&score.rubric_criterion_id)
            .ok_or_else(|| anyhow!("Evaluation criterion is outside the pinned rubric."))?;
        if score.score > *max_points {
            return Err(anyhow!("Criterion score exceeds its rubric maximum."));
        }
        total_score += score.score;
        total_max += max_points;
    }
    let normalized_score = total_score / total_max;
    if !matches!(request.outcome, AssessmentOutcome::Inconclusive)
        && ((request.outcome == AssessmentOutcome::Pass) != (normalized_score >= pass_threshold))
    {
        return Err(anyhow!(
            "Pass/fail outcome must agree with the pinned rubric threshold; use inconclusive when appropriate."
        ));
    }
    let prior: Option<(String, i64)> = tx
        .query_row(
            "SELECT id, revision_no FROM attempt_evaluations
             WHERE attempt_id = ?1 ORDER BY revision_no DESC LIMIT 1",
            [&request.attempt_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (revision_no, correction_of_id) = match (prior, &request.correction_of_id) {
        (None, None) => (1, None),
        (Some((previous_id, previous_revision)), Some(correction_id))
            if correction_id == &previous_id =>
        {
            (previous_revision + 1, Some(previous_id))
        }
        (None, Some(_)) => return Err(anyhow!("An initial evaluation cannot correct a prior record.")),
        (Some(_), None) => return Err(anyhow!("A later evaluation must explicitly correct the latest evaluation.")),
        (Some(_), Some(_)) => return Err(anyhow!("Evaluation correction must target the latest revision.")),
    };
    let evaluation_id = id();
    tx.execute(
        "INSERT INTO attempt_evaluations
            (id, attempt_id, rubric_version_id, revision_no, correction_of_id,
             evaluator_id, outcome, score, rationale, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            evaluation_id,
            request.attempt_id,
            rubric_version_id,
            revision_no,
            correction_of_id,
            request.evaluator_id.trim(),
            request.outcome.as_str(),
            normalized_score,
            request.rationale.trim(),
            now
        ],
    )?;
    for score in &request.criterion_scores {
        tx.execute(
            "INSERT INTO attempt_evaluation_scores
                (evaluation_id, rubric_criterion_id, score) VALUES (?1, ?2, ?3)",
            params![evaluation_id, score.rubric_criterion_id, score.score],
        )?;
    }
    tx.execute(
        "INSERT INTO evaluator_feedback
            (id, evaluation_id, feedback, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id(),
            evaluation_id,
            request.feedback.trim(),
            request.evaluator_id.trim(),
            now
        ],
    )?;
    tx.commit()?;
    query_attempt_evaluation(conn, &evaluation_id)
}

pub fn create_competency(
    conn: &mut Connection,
    request: &CreateCompetencyReq,
) -> Result<CompetencyDto> {
    required(&request.title, "Competency title")?;
    if request.criteria.is_empty() || !request.criteria.iter().any(|criterion| criterion.required) {
        return Err(anyhow!("A competency needs at least one required criterion."));
    }
    let (_, framework_id, framework_version_id) = project_context(conn, &request.learning_project_id)?;
    let competency_id = id();
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    tx.execute(
        "INSERT INTO competencies
            (id, learning_project_id, framework_id, framework_version_id,
             title, description, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            competency_id,
            request.learning_project_id,
            framework_id,
            framework_version_id,
            request.title.trim(),
            request.description.trim(),
            request.created_by,
            now_us()?
        ],
    )?;
    for criterion in &request.criteria {
        required(&criterion.criterion, "Competency criterion")?;
        valid_unit_score(criterion.minimum_score, "Competency criterion threshold")?;
        tx.execute(
            "INSERT INTO competency_criteria
                (id, competency_id, learning_project_id, framework_id,
                 framework_version_id, objective_version_id, criterion,
                 minimum_score, required)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id(),
                competency_id,
                request.learning_project_id,
                framework_id,
                framework_version_id,
                criterion.objective_version_id,
                criterion.criterion.trim(),
                criterion.minimum_score,
                i64::from(criterion.required)
            ],
        )?;
    }
    tx.commit()?;
    query_competency(conn, &competency_id)
}

pub fn create_mastery_rule_version(
    conn: &mut Connection,
    request: &CreateMasteryRuleVersionReq,
) -> Result<MasteryRuleDto> {
    required(&request.title, "Mastery rule title")?;
    valid_unit_score(request.minimum_average_score, "Mastery average score threshold")?;
    if request.minimum_evaluated_attempts <= 0 {
        return Err(anyhow!("Minimum evaluated attempts must be positive."));
    }
    if request.reproduction_required && request.required_reproduction_outcome != AssessmentOutcome::Pass {
        return Err(anyhow!(
            "Mastery requiring independent reproduction must require a passing evaluated reproduction."
        ));
    }
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let (rule_id, version_no) = if let Some(rule_id) = &request.mastery_rule_id {
        let competency_id: String = tx.query_row(
            "SELECT competency_id FROM mastery_rules WHERE id = ?1",
            [rule_id],
            |row| row.get(0),
        )?;
        if competency_id != request.competency_id {
            return Err(anyhow!("Mastery rule versions cannot change their competency."));
        }
        let version_no: i64 = tx.query_row(
            "SELECT COALESCE(MAX(version_no), 0) + 1
             FROM mastery_rule_versions WHERE mastery_rule_id = ?1",
            [rule_id],
            |row| row.get(0),
        )?;
        (rule_id.clone(), version_no)
    } else {
        let rule_id = id();
        tx.execute(
            "INSERT INTO mastery_rules (id, competency_id, title, created_by, created_at_us)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                rule_id,
                request.competency_id,
                request.title.trim(),
                request.created_by,
                now_us()?
            ],
        )?;
        (rule_id, 1)
    };
    let rule_version_id = id();
    tx.execute(
        "INSERT INTO mastery_rule_versions
            (id, mastery_rule_id, competency_id, version_no, minimum_average_score,
             minimum_evaluated_attempts, all_required_criteria, reproduction_required,
             required_reproduction_outcome, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            rule_version_id,
            rule_id,
            request.competency_id,
            version_no,
            request.minimum_average_score,
            request.minimum_evaluated_attempts,
            i64::from(request.all_required_criteria),
            i64::from(request.reproduction_required),
            request.required_reproduction_outcome.as_str(),
            request.created_by,
            now_us()?
        ],
    )?;
    tx.commit()?;
    query_mastery_rule(conn, &rule_id)
}

pub fn evaluate_competency(
    conn: &mut Connection,
    request: &EvaluateCompetencyReq,
) -> Result<CompetencyRecordDto> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let (learning_project_id, framework_version_id, learner_id): (String, String, String) =
        tx.query_row(
            "SELECT c.learning_project_id, c.framework_version_id, p.learner_id
             FROM competencies c JOIN learning_projects p ON p.id = c.learning_project_id
             WHERE c.id = ?1",
            [&request.competency_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    if learner_id != request.learner_id {
        return Err(anyhow!("Competency evaluation learner does not own this learning project."));
    }
    let rule: (String, f64, i64, bool, bool, String) = tx.query_row(
        "SELECT competency_id, minimum_average_score, minimum_evaluated_attempts,
                all_required_criteria, reproduction_required, required_reproduction_outcome
         FROM mastery_rule_versions WHERE id = ?1",
        [&request.mastery_rule_version_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get::<_, i64>(3)? != 0,
                row.get::<_, i64>(4)? != 0,
                row.get(5)?,
            ))
        },
    )?;
    if rule.0 != request.competency_id {
        return Err(anyhow!("Mastery rule version targets a different competency."));
    }
    let criteria = query_competency(&tx, &request.competency_id)?.criteria;
    let mut statement = tx.prepare(
        "SELECT e.id, e.attempt_id, e.score, e.outcome
         FROM attempt_evaluations e
         JOIN assessment_attempts a ON a.id = e.attempt_id
         WHERE a.learning_project_id = ?1 AND a.learner_id = ?2
           AND a.framework_version_id = ?3 AND a.status = 'submitted'
           AND EXISTS (
               SELECT 1 FROM assessment_version_objectives avo
               JOIN competency_criteria cc ON cc.objective_version_id = avo.objective_version_id
               WHERE avo.assessment_version_id = a.assessment_version_id
                 AND cc.competency_id = ?4
           )
           AND NOT EXISTS (
               SELECT 1 FROM attempt_evaluations later
               WHERE later.attempt_id = e.attempt_id AND later.revision_no > e.revision_no
           )
         ORDER BY e.created_at_us, e.id",
    )?;
    let attempt_evaluations = statement
        .query_map(
            params![
                learning_project_id,
                request.learner_id,
                framework_version_id,
                request.competency_id
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, f64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);

    let mut criterion_results = Vec::new();
    for criterion in &criteria {
        let satisfied: bool = tx.query_row(
            "SELECT EXISTS (
                SELECT 1 FROM attempt_evaluations e
                JOIN assessment_attempts a ON a.id = e.attempt_id
                JOIN attempt_evaluation_scores s ON s.evaluation_id = e.id
                JOIN assessment_rubric_criteria rc
                  ON rc.id = s.rubric_criterion_id
                 AND rc.objective_version_id = ?4
                WHERE a.learning_project_id = ?1 AND a.learner_id = ?2
                  AND a.framework_version_id = ?3 AND a.status = 'submitted'
                  AND s.score / rc.max_points >= ?5
                  AND NOT EXISTS (
                      SELECT 1 FROM attempt_evaluations later
                      WHERE later.attempt_id = e.attempt_id
                        AND later.revision_no > e.revision_no
                  )
            )",
            params![
                learning_project_id,
                request.learner_id,
                framework_version_id,
                criterion.objective_version_id,
                criterion.minimum_score
            ],
            |row| row.get(0),
        )?;
        criterion_results.push((criterion.clone(), satisfied));
    }

    let reproduction_evaluations = {
        let mut statement = tx.prepare(
            "SELECT e.id, e.outcome FROM reproduction_evaluations e
             JOIN reproduction_submissions s ON s.id = e.submission_id
             JOIN reproduction_tasks t ON t.id = s.task_id
             WHERE s.learner_id = ?1 AND t.learning_project_id = ?2
               AND t.framework_version_id = ?3
               AND t.objective_version_id IN (
                   SELECT objective_version_id FROM competency_criteria
                   WHERE competency_id = ?4
               )
             ORDER BY e.created_at_us, e.id",
        )?;
        let rows = statement
            .query_map(
                params![
                    request.learner_id,
                    learning_project_id,
                    framework_version_id,
                    request.competency_id
                ],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    let average_score = if attempt_evaluations.is_empty() {
        0.0
    } else {
        attempt_evaluations.iter().map(|evaluation| evaluation.2).sum::<f64>()
            / attempt_evaluations.len() as f64
    };
    let minimum_attempts_met =
        attempt_evaluations.len() >= usize::try_from(rule.2).unwrap_or(usize::MAX);
    let required_criteria_satisfied = criterion_results
        .iter()
        .filter(|(criterion, _)| criterion.required)
        .all(|(_, satisfied)| *satisfied);
    let all_criteria_satisfied = criterion_results.iter().all(|(_, satisfied)| *satisfied);
    let criteria_satisfied = if rule.3 {
        all_criteria_satisfied
    } else {
        required_criteria_satisfied
    };
    let reproduction_satisfied = !rule.4
        || reproduction_evaluations
            .iter()
            .any(|(_, outcome)| outcome == &rule.5);
    let score_satisfied = average_score >= rule.1;
    let mastered = minimum_attempts_met
        && criteria_satisfied
        && reproduction_satisfied
        && score_satisfied;
    let outcome = if mastered {
        MasteryOutcome::Mastered
    } else {
        MasteryOutcome::NotMastered
    };
    let mut explanation = format!(
        "Average evaluated score {:.3} / {:.3}; {} qualifying assessment evaluations (minimum {}). ",
        average_score,
        rule.1,
        attempt_evaluations.len(),
        rule.2
    );
    if criteria_satisfied {
        explanation.push_str("Required competency criteria are satisfied. ");
    } else {
        let unmet = criterion_results
            .iter()
            .filter(|(criterion, satisfied)| {
                (rule.3 || criterion.required) && !satisfied
            })
            .map(|(criterion, _)| criterion.criterion.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        explanation.push_str(&format!("Unmet criteria: {unmet}. "));
    }
    if rule.4 {
        explanation.push_str(if reproduction_satisfied {
            "A passing independent reproduction evaluation is present. "
        } else {
            "A passing independent reproduction evaluation is required. "
        });
    }
    if !minimum_attempts_met {
        explanation.push_str("The minimum number of evaluated attempts has not been reached. ");
    }
    if !score_satisfied {
        explanation.push_str("The minimum average score has not been reached.");
    }

    let record_id = id();
    let created_at = now_us()?;
    tx.execute(
        "INSERT INTO competency_records
            (id, competency_id, learning_project_id, learner_id, framework_version_id,
             mastery_rule_version_id, outcome, explanation, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            record_id,
            request.competency_id,
            learning_project_id,
            request.learner_id,
            framework_version_id,
            request.mastery_rule_version_id,
            outcome.as_str(),
            explanation,
            created_at
        ],
    )?;
    for (criterion, satisfied) in &criterion_results {
        tx.execute(
            "INSERT INTO competency_record_criteria
                (competency_record_id, competency_criterion_id, satisfied, rationale)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                record_id,
                criterion.id,
                i64::from(*satisfied),
                if *satisfied {
                    "Criterion threshold met by pinned assessment evidence."
                } else {
                    "No pinned assessment criterion score met the configured threshold."
                }
            ],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO competency_record_objectives
                (competency_record_id, objective_version_id) VALUES (?1, ?2)",
            params![record_id, criterion.objective_version_id],
        )?;
    }
    for (evaluation_id, _, _, _) in &attempt_evaluations {
        tx.execute(
            "INSERT INTO competency_record_attempt_evaluations
                (competency_record_id, evaluation_id) VALUES (?1, ?2)",
            params![record_id, evaluation_id],
        )?;
    }
    for (evaluation_id, _) in &reproduction_evaluations {
        tx.execute(
            "INSERT INTO competency_record_reproduction_evaluations
                (competency_record_id, evaluation_id) VALUES (?1, ?2)",
            params![record_id, evaluation_id],
        )?;
    }
    tx.commit()?;
    query_competency_record(conn, &record_id)
}

pub fn create_reproduction_task(
    conn: &mut Connection,
    request: &CreateReproductionTaskReq,
) -> Result<ReproductionTaskDto> {
    required(&request.title, "Reproduction task title")?;
    required(&request.task_description, "Reproduction task description")?;
    required(&request.conditions, "Reproduction conditions")?;
    if request.expected_result.is_null() || request.expected_result == serde_json::json!({}) {
        return Err(anyhow!("Reproduction task requires a non-empty expected result."));
    }
    let (_, framework_id, framework_version_id) = project_context(conn, &request.learning_project_id)?;
    let task_id = id();
    let expected_result = json(&request.expected_result)?;
    conn.execute(
        "INSERT INTO reproduction_tasks
            (id, learning_project_id, framework_id, framework_version_id,
             objective_version_id, rubric_version_id, title, task_description,
             conditions, expected_result_json, created_by, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            task_id,
            request.learning_project_id,
            framework_id,
            framework_version_id,
            request.objective_version_id,
            request.rubric_version_id,
            request.title.trim(),
            request.task_description.trim(),
            request.conditions.trim(),
            expected_result,
            request.created_by,
            now_us()?
        ],
    )?;
    query_reproduction_task(conn, &task_id)
}

pub fn submit_reproduction(
    conn: &mut Connection,
    request: &SubmitReproductionReq,
) -> Result<ReproductionSubmissionDto> {
    required(&request.learner_id, "Reproduction learner identity")?;
    let uri = request.artifact_uri.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let checksum = request
        .artifact_sha256
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if uri.is_some() != checksum.is_some() {
        return Err(anyhow!("A reproduction artifact requires both URI and SHA-256 checksum."));
    }
    if let Some(value) = checksum {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(anyhow!("Reproduction artifact checksum must be a SHA-256 digest."));
        }
    }
    let result_json = request.result.as_ref().map(json).transpose()?;
    let has_result = request
        .result
        .as_ref()
        .is_some_and(|result| !result.is_null() && result != &serde_json::json!({}));
    if uri.is_none() && !has_result {
        return Err(anyhow!("A reproduction submission requires an artifact or a non-empty result."));
    }
    let submission_id = id();
    conn.execute(
        "INSERT INTO reproduction_submissions
            (id, task_id, learner_id, artifact_uri, artifact_sha256,
             result_json, deviations, submitted_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            submission_id,
            request.task_id,
            request.learner_id,
            uri,
            checksum.map(|value| value.to_ascii_lowercase()),
            result_json,
            request.deviations.trim(),
            now_us()?
        ],
    )?;
    query_reproduction_submission(conn, &submission_id)
}

pub fn evaluate_reproduction(
    conn: &mut Connection,
    request: &EvaluateReproductionReq,
) -> Result<ReproductionEvaluationDto> {
    required(&request.evaluator_id, "Reproduction evaluator identity")?;
    required(&request.rationale, "Reproduction evaluation rationale")?;
    required(&request.evidence_description, "Reproduction evidence description")?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let (learner_id, rubric_version_id, pass_threshold): (String, String, f64) = tx.query_row(
        "SELECT s.learner_id, t.rubric_version_id, rv.pass_threshold
         FROM reproduction_submissions s
         JOIN reproduction_tasks t ON t.id = s.task_id
         JOIN assessment_rubric_versions rv ON rv.id = t.rubric_version_id
         WHERE s.id = ?1",
        [&request.submission_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    if request.evaluator_id.trim() == learner_id {
        return Err(anyhow!("Independent reproduction requires an evaluator distinct from the learner."));
    }
    let score = validate_rubric_scores(
        &tx,
        &rubric_version_id,
        &request.criterion_scores,
        pass_threshold,
        request.outcome,
    )?;
    let evaluation_id = id();
    let created_at = now_us()?;
    tx.execute(
        "INSERT INTO reproduction_evaluations
            (id, submission_id, rubric_version_id, evaluator_id, outcome, score,
             rationale, evidence_description, created_at_us)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            evaluation_id,
            request.submission_id,
            rubric_version_id,
            request.evaluator_id.trim(),
            request.outcome.as_str(),
            score,
            request.rationale.trim(),
            request.evidence_description.trim(),
            created_at
        ],
    )?;
    insert_reproduction_scores(&tx, &evaluation_id, &request.criterion_scores)?;
    tx.commit()?;
    query_reproduction_evaluation(conn, &evaluation_id)
}

fn validate_rubric_scores(
    conn: &Connection,
    rubric_version_id: &str,
    scores: &[EvaluationCriterionScoreDto],
    pass_threshold: f64,
    outcome: AssessmentOutcome,
) -> Result<f64> {
    let mut seen = HashSet::new();
    for item in scores {
        if !item.score.is_finite() || item.score < 0.0 || !seen.insert(&item.rubric_criterion_id) {
            return Err(anyhow!("Rubric scores must be unique finite non-negative values."));
        }
    }
    let criteria: HashMap<String, f64> = {
        let mut statement = conn.prepare(
            "SELECT id, max_points FROM assessment_rubric_criteria WHERE rubric_version_id = ?1",
        )?;
        let rows = statement
            .query_map([rubric_version_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<HashMap<_, _>>>()?;
        rows
    };
    if criteria.is_empty() || criteria.len() != scores.len() {
        return Err(anyhow!("Every pinned rubric criterion must be scored exactly once."));
    }
    let mut total_score = 0.0;
    let mut total_max = 0.0;
    for score in scores {
        let max = criteria
            .get(&score.rubric_criterion_id)
            .ok_or_else(|| anyhow!("Score references a criterion outside the pinned rubric."))?;
        if score.score > *max {
            return Err(anyhow!("Rubric criterion score exceeds its maximum."));
        }
        total_score += score.score;
        total_max += *max;
    }
    let normalized = total_score / total_max;
    if outcome != AssessmentOutcome::Inconclusive
        && ((outcome == AssessmentOutcome::Pass) != (normalized >= pass_threshold))
    {
        return Err(anyhow!("Evaluation outcome does not agree with the pinned rubric threshold."));
    }
    Ok(normalized)
}

fn insert_reproduction_scores(
    tx: &Transaction<'_>,
    evaluation_id: &str,
    scores: &[EvaluationCriterionScoreDto],
) -> Result<()> {
    for score in scores {
        tx.execute(
            "INSERT INTO reproduction_evaluation_scores
                (evaluation_id, rubric_criterion_id, score) VALUES (?1, ?2, ?3)",
            params![evaluation_id, score.rubric_criterion_id, score.score],
        )?;
    }
    Ok(())
}

pub fn workspace(conn: &Connection) -> Result<LearningWorkspaceDto> {
    let learners = query_many(conn, "SELECT id FROM learning_learners ORDER BY display_name, id", query_learner)?;
    let projects = query_many(conn, "SELECT id FROM learning_projects ORDER BY created_at_us DESC, id", query_project)?;
    let framework_versions = {
        let mut statement = conn.prepare(
            "SELECT f.id, v.id, v.title, v.version_no, v.published_at_us
             FROM knowledge_framework_versions v
             JOIN knowledge_frameworks f ON f.id = v.framework_id
             WHERE v.status = 'published'
             ORDER BY v.title, v.version_no DESC",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok(LearningFrameworkVersionDto {
                    framework_id: row.get(0)?,
                    framework_version_id: row.get(1)?,
                    title: row.get(2)?,
                    version_no: row.get(3)?,
                    published_at_us: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    let objectives = query_many(
        conn,
        "SELECT id FROM learning_objective_versions ORDER BY created_at_us, id",
        query_objective,
    )?;
    let assessments = query_many(
        conn,
        "SELECT id FROM assessment_versions ORDER BY created_at_us DESC, id",
        query_assessment_version,
    )?;
    let attempts = query_many(
        conn,
        "SELECT id FROM assessment_attempts ORDER BY started_at_us DESC, id",
        query_attempt,
    )?;
    let competencies = query_many(
        conn,
        "SELECT id FROM competencies ORDER BY created_at_us, id",
        query_competency,
    )?;
    let mastery_rules = query_many(
        conn,
        "SELECT id FROM mastery_rules ORDER BY created_at_us, id",
        query_mastery_rule,
    )?;
    let competency_records = query_many(
        conn,
        "SELECT id FROM competency_records ORDER BY created_at_us DESC, id",
        query_competency_record,
    )?;
    let reproduction_tasks = query_many(
        conn,
        "SELECT id FROM reproduction_tasks ORDER BY created_at_us DESC, id",
        query_reproduction_task,
    )?;
    let reproduction_submissions = query_many(
        conn,
        "SELECT id FROM reproduction_submissions ORDER BY submitted_at_us DESC, id",
        query_reproduction_submission,
    )?;
    let progress = query_progress(conn)?;
    Ok(LearningWorkspaceDto {
        learners,
        projects,
        framework_versions,
        objectives,
        assessments,
        attempts,
        competencies,
        mastery_rules,
        competency_records,
        reproduction_tasks,
        reproduction_submissions,
        progress,
    })
}

fn query_many<T>(
    conn: &Connection,
    id_sql: &str,
    mapper: fn(&Connection, &str) -> Result<T>,
) -> Result<Vec<T>> {
    let ids = {
        let mut statement = conn.prepare(id_sql)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    ids.iter().map(|record_id| mapper(conn, record_id)).collect()
}

fn query_learner(conn: &Connection, learner_id: &str) -> Result<LearningLearnerDto> {
    conn.query_row(
        "SELECT id, display_name, created_by, created_at_us
         FROM learning_learners WHERE id = ?1",
        [learner_id],
        |row| {
            Ok(LearningLearnerDto {
                id: row.get(0)?,
                display_name: row.get(1)?,
                created_by: row.get(2)?,
                created_at_us: row.get(3)?,
            })
        },
    )
    .map_err(Into::into)
}

fn query_project(conn: &Connection, project_id: &str) -> Result<LearningProjectDto> {
    conn.query_row(
        "SELECT p.id, p.title, p.learner_id, l.display_name, p.framework_id,
                p.framework_version_id, v.title, v.version_no, p.created_by, p.created_at_us
         FROM learning_projects p
         JOIN learning_learners l ON l.id = p.learner_id
         JOIN knowledge_framework_versions v ON v.id = p.framework_version_id
         WHERE p.id = ?1",
        [project_id],
        |row| {
            Ok(LearningProjectDto {
                id: row.get(0)?,
                title: row.get(1)?,
                learner_id: row.get(2)?,
                learner_name: row.get(3)?,
                framework_id: row.get(4)?,
                framework_version_id: row.get(5)?,
                framework_title: row.get(6)?,
                framework_version_no: row.get(7)?,
                created_by: row.get(8)?,
                created_at_us: row.get(9)?,
            })
        },
    )
    .map_err(Into::into)
}

fn query_objective(conn: &Connection, version_id: &str) -> Result<LearningObjectiveVersionDto> {
    let mut dto = conn.query_row(
        "SELECT ov.id, ov.objective_id, ov.version_no, ov.learning_project_id,
                ov.framework_id, ov.framework_version_id, ov.parent_version_id,
                o.title, ov.observable_outcome, ov.criteria_text, ov.level,
                ov.created_by, ov.created_at_us
         FROM learning_objective_versions ov
         JOIN learning_objectives o ON o.id = ov.objective_id
         WHERE ov.id = ?1",
        [version_id],
        |row| {
            Ok(LearningObjectiveVersionDto {
                id: row.get(0)?,
                objective_id: row.get(1)?,
                version_no: row.get(2)?,
                learning_project_id: row.get(3)?,
                framework_id: row.get(4)?,
                framework_version_id: row.get(5)?,
                parent_version_id: row.get(6)?,
                title: row.get(7)?,
                observable_outcome: row.get(8)?,
                criteria_text: row.get(9)?,
                level: row.get(10)?,
                prerequisite_version_ids: Vec::new(),
                created_by: row.get(11)?,
                created_at_us: row.get(12)?,
            })
        },
    )?;
    dto.prerequisite_version_ids = {
        let mut statement = conn.prepare(
            "SELECT prerequisite_version_id FROM learning_objective_prerequisites
             WHERE objective_version_id = ?1 ORDER BY prerequisite_version_id",
        )?;
        let rows = statement
            .query_map([version_id], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    Ok(dto)
}

fn query_assessment_version(conn: &Connection, version_id: &str) -> Result<AssessmentVersionDto> {
    let (assessment, status): (String, String) = conn.query_row(
        "SELECT a.title, av.status FROM assessment_versions av
         JOIN assessments a ON a.id = av.assessment_id WHERE av.id = ?1",
        [version_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let mut dto = conn.query_row(
        "SELECT av.id, av.assessment_id, av.version_no, av.learning_project_id,
                av.framework_id, av.framework_version_id, av.rubric_version_id,
                rv.pass_threshold, av.status, av.change_rationale, av.created_by,
                av.created_at_us, av.published_at_us
         FROM assessment_versions av
         JOIN assessment_rubric_versions rv ON rv.id = av.rubric_version_id
         WHERE av.id = ?1",
        [version_id],
        |row| {
            Ok(AssessmentVersionDto {
                id: row.get(0)?,
                assessment_id: row.get(1)?,
                title: assessment,
                version_no: row.get(2)?,
                learning_project_id: row.get(3)?,
                framework_id: row.get(4)?,
                framework_version_id: row.get(5)?,
                rubric_version_id: row.get(6)?,
                rubric_pass_threshold: row.get(7)?,
                status: LearningVersionStatus::parse(&status)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                change_rationale: row.get(9)?,
                created_by: row.get(10)?,
                created_at_us: row.get(11)?,
                published_at_us: row.get(12)?,
                questions: Vec::new(),
                rubric_criteria: Vec::new(),
                objective_version_ids: Vec::new(),
                reviews: Vec::new(),
            })
        },
    )?;
    dto.questions = {
        let mut statement = conn.prepare(
            "SELECT id, objective_version_id, question_order, prompt, response_kind,
                    max_points, required
             FROM assessment_questions WHERE assessment_version_id = ?1
             ORDER BY question_order, id",
        )?;
        let rows = statement
            .query_map([version_id], |row| {
                Ok(AssessmentQuestionDto {
                    id: row.get(0)?,
                    objective_version_id: row.get(1)?,
                    question_order: row.get(2)?,
                    prompt: row.get(3)?,
                    response_kind: row.get(4)?,
                    max_points: row.get(5)?,
                    required: row.get::<_, i64>(6)? != 0,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    dto.rubric_criteria = {
        let mut statement = conn.prepare(
            "SELECT id, objective_version_id, criterion_key, description, max_points
             FROM assessment_rubric_criteria WHERE rubric_version_id = ?1
             ORDER BY criterion_key, id",
        )?;
        let rows = statement
            .query_map([&dto.rubric_version_id], |row| {
                Ok(AssessmentRubricCriterionDto {
                    id: row.get(0)?,
                    objective_version_id: row.get(1)?,
                    criterion_key: row.get(2)?,
                    description: row.get(3)?,
                    max_points: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    dto.objective_version_ids = {
        let mut statement = conn.prepare(
            "SELECT objective_version_id FROM assessment_version_objectives
             WHERE assessment_version_id = ?1 ORDER BY objective_version_id",
        )?;
        let rows = statement
            .query_map([version_id], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    dto.reviews = {
        let mut statement = conn.prepare(
            "SELECT id, review_status, reviewer_id, rationale, reviewed_at_us
             FROM assessment_reviews WHERE assessment_version_id = ?1
             ORDER BY reviewed_at_us, id",
        )?;
        let rows = statement
            .query_map([version_id], |row| {
                let review_status: String = row.get(1)?;
                let review_status = match review_status.as_str() {
                    "approved" => AssessmentReviewStatus::Approved,
                    "changes_requested" => AssessmentReviewStatus::ChangesRequested,
                    "rejected" => AssessmentReviewStatus::Rejected,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                Ok(AssessmentReviewDto {
                    id: row.get(0)?,
                    review_status,
                    reviewer_id: row.get(2)?,
                    rationale: row.get(3)?,
                    reviewed_at_us: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    Ok(dto)
}

fn query_attempt(conn: &Connection, attempt_id: &str) -> Result<AssessmentAttemptDto> {
    let mut dto = conn.query_row(
        "SELECT id, learning_project_id, learner_id, assessment_version_id,
                framework_version_id, status, started_at_us, submitted_at_us
         FROM assessment_attempts WHERE id = ?1",
        [attempt_id],
        |row| {
            Ok(AssessmentAttemptDto {
                id: row.get(0)?,
                learning_project_id: row.get(1)?,
                learner_id: row.get(2)?,
                assessment_version_id: row.get(3)?,
                framework_version_id: row.get(4)?,
                status: row.get(5)?,
                started_at_us: row.get(6)?,
                submitted_at_us: row.get(7)?,
                responses: Vec::new(),
                evaluations: Vec::new(),
            })
        },
    )?;
    dto.responses = {
        let mut statement = conn.prepare(
            "SELECT id, assessment_question_id, response_json, updated_at_us
             FROM attempt_responses WHERE attempt_id = ?1 ORDER BY created_at_us, id",
        )?;
        let rows = statement
            .query_map([attempt_id], |row| {
                Ok(AttemptResponseDto {
                    id: row.get(0)?,
                    assessment_question_id: row.get(1)?,
                    response: parse_json(&row.get::<_, String>(2)?)?,
                    updated_at_us: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    let evaluation_ids = query_ids(
        conn,
        "SELECT id FROM attempt_evaluations WHERE attempt_id = ?1 ORDER BY revision_no",
        attempt_id,
    )?;
    dto.evaluations = evaluation_ids
        .iter()
        .map(|evaluation_id| query_attempt_evaluation(conn, evaluation_id))
        .collect::<Result<Vec<_>>>()?;
    Ok(dto)
}

fn query_attempt_evaluation(conn: &Connection, evaluation_id: &str) -> Result<AttemptEvaluationDto> {
    let mut dto = conn.query_row(
        "SELECT id, attempt_id, rubric_version_id, revision_no, correction_of_id,
                evaluator_id, outcome, score, rationale, created_at_us
         FROM attempt_evaluations WHERE id = ?1",
        [evaluation_id],
        |row| {
            let outcome: String = row.get(6)?;
            Ok(AttemptEvaluationDto {
                id: row.get(0)?,
                attempt_id: row.get(1)?,
                rubric_version_id: row.get(2)?,
                revision_no: row.get(3)?,
                correction_of_id: row.get(4)?,
                evaluator_id: row.get(5)?,
                outcome: parse_outcome(&outcome)?,
                score: row.get(7)?,
                rationale: row.get(8)?,
                criterion_scores: Vec::new(),
                feedback: Vec::new(),
                created_at_us: row.get(9)?,
            })
        },
    )?;
    dto.criterion_scores = query_score_rows(
        conn,
        "SELECT rubric_criterion_id, score FROM attempt_evaluation_scores
         WHERE evaluation_id = ?1 ORDER BY rubric_criterion_id",
        evaluation_id,
    )?;
    dto.feedback = {
        let mut statement = conn.prepare(
            "SELECT feedback FROM evaluator_feedback WHERE evaluation_id = ?1
             ORDER BY created_at_us, id",
        )?;
        let rows = statement
            .query_map([evaluation_id], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    Ok(dto)
}

fn parse_outcome(value: &str) -> rusqlite::Result<AssessmentOutcome> {
    match value {
        "pass" => Ok(AssessmentOutcome::Pass),
        "fail" => Ok(AssessmentOutcome::Fail),
        "inconclusive" => Ok(AssessmentOutcome::Inconclusive),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn query_score_rows(
    conn: &Connection,
    sql: &str,
    parent_id: &str,
) -> Result<Vec<EvaluationCriterionScoreDto>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement
        .query_map([parent_id], |row| {
            Ok(EvaluationCriterionScoreDto {
                rubric_criterion_id: row.get(0)?,
                score: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn query_competency(conn: &Connection, competency_id: &str) -> Result<CompetencyDto> {
    let mut dto = conn.query_row(
        "SELECT id, learning_project_id, framework_id, framework_version_id,
                title, description FROM competencies WHERE id = ?1",
        [competency_id],
        |row| {
            Ok(CompetencyDto {
                id: row.get(0)?,
                learning_project_id: row.get(1)?,
                framework_id: row.get(2)?,
                framework_version_id: row.get(3)?,
                title: row.get(4)?,
                description: row.get(5)?,
                criteria: Vec::new(),
            })
        },
    )?;
    let mut statement = conn.prepare(
        "SELECT id, objective_version_id, criterion, minimum_score, required
         FROM competency_criteria WHERE competency_id = ?1 ORDER BY id",
    )?;
    dto.criteria = statement
        .query_map([competency_id], |row| {
            Ok(CompetencyCriterionDto {
                id: row.get(0)?,
                objective_version_id: row.get(1)?,
                criterion: row.get(2)?,
                minimum_score: row.get(3)?,
                required: row.get::<_, i64>(4)? != 0,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(dto)
}

fn query_mastery_rule(conn: &Connection, rule_id: &str) -> Result<MasteryRuleDto> {
    let mut dto = conn.query_row(
        "SELECT id, competency_id, title FROM mastery_rules WHERE id = ?1",
        [rule_id],
        |row| {
            Ok(MasteryRuleDto {
                id: row.get(0)?,
                competency_id: row.get(1)?,
                title: row.get(2)?,
                versions: Vec::new(),
            })
        },
    )?;
    let mut statement = conn.prepare(
        "SELECT id, mastery_rule_id, competency_id, version_no, minimum_average_score,
                minimum_evaluated_attempts, all_required_criteria, reproduction_required,
                required_reproduction_outcome, created_by, created_at_us
         FROM mastery_rule_versions WHERE mastery_rule_id = ?1 ORDER BY version_no",
    )?;
    dto.versions = statement
        .query_map([rule_id], |row| {
            let outcome: String = row.get(8)?;
            Ok(MasteryRuleVersionDto {
                id: row.get(0)?,
                mastery_rule_id: row.get(1)?,
                competency_id: row.get(2)?,
                version_no: row.get(3)?,
                minimum_average_score: row.get(4)?,
                minimum_evaluated_attempts: row.get(5)?,
                all_required_criteria: row.get::<_, i64>(6)? != 0,
                reproduction_required: row.get::<_, i64>(7)? != 0,
                required_reproduction_outcome: parse_outcome(&outcome)?,
                created_by: row.get(9)?,
                created_at_us: row.get(10)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(dto)
}

fn query_competency_record(conn: &Connection, record_id: &str) -> Result<CompetencyRecordDto> {
    let mut dto = conn.query_row(
        "SELECT id, competency_id, learning_project_id, learner_id, framework_version_id,
                mastery_rule_version_id, outcome, explanation, created_at_us
         FROM competency_records WHERE id = ?1",
        [record_id],
        |row| {
            let outcome: String = row.get(6)?;
            let outcome = match outcome.as_str() {
                "mastered" => MasteryOutcome::Mastered,
                "not_mastered" => MasteryOutcome::NotMastered,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            Ok(CompetencyRecordDto {
                id: row.get(0)?,
                competency_id: row.get(1)?,
                learning_project_id: row.get(2)?,
                learner_id: row.get(3)?,
                framework_version_id: row.get(4)?,
                mastery_rule_version_id: row.get(5)?,
                outcome,
                explanation: row.get(7)?,
                attempt_evaluation_ids: Vec::new(),
                reproduction_evaluation_ids: Vec::new(),
                objective_version_ids: Vec::new(),
                criteria: Vec::new(),
                created_at_us: row.get(8)?,
            })
        },
    )?;
    dto.attempt_evaluation_ids = query_ids(
        conn,
        "SELECT evaluation_id FROM competency_record_attempt_evaluations
         WHERE competency_record_id = ?1 ORDER BY evaluation_id",
        record_id,
    )?;
    dto.reproduction_evaluation_ids = query_ids(
        conn,
        "SELECT evaluation_id FROM competency_record_reproduction_evaluations
         WHERE competency_record_id = ?1 ORDER BY evaluation_id",
        record_id,
    )?;
    dto.objective_version_ids = query_ids(
        conn,
        "SELECT objective_version_id FROM competency_record_objectives
         WHERE competency_record_id = ?1 ORDER BY objective_version_id",
        record_id,
    )?;
    let mut statement = conn.prepare(
        "SELECT competency_criterion_id, satisfied, rationale
         FROM competency_record_criteria WHERE competency_record_id = ?1
         ORDER BY competency_criterion_id",
    )?;
    dto.criteria = statement
        .query_map([record_id], |row| {
            Ok(CompetencyRecordCriterionDto {
                competency_criterion_id: row.get(0)?,
                satisfied: row.get::<_, i64>(1)? != 0,
                rationale: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(dto)
}

fn query_reproduction_task(conn: &Connection, task_id: &str) -> Result<ReproductionTaskDto> {
    conn.query_row(
        "SELECT id, learning_project_id, framework_id, framework_version_id,
                objective_version_id, rubric_version_id, title, task_description,
                conditions, expected_result_json, created_by, created_at_us
         FROM reproduction_tasks WHERE id = ?1",
        [task_id],
        |row| {
            Ok(ReproductionTaskDto {
                id: row.get(0)?,
                learning_project_id: row.get(1)?,
                framework_id: row.get(2)?,
                framework_version_id: row.get(3)?,
                objective_version_id: row.get(4)?,
                rubric_version_id: row.get(5)?,
                title: row.get(6)?,
                task_description: row.get(7)?,
                conditions: row.get(8)?,
                expected_result: parse_json(&row.get::<_, String>(9)?)?,
                created_by: row.get(10)?,
                created_at_us: row.get(11)?,
            })
        },
    )
    .map_err(Into::into)
}

fn query_reproduction_submission(
    conn: &Connection,
    submission_id: &str,
) -> Result<ReproductionSubmissionDto> {
    let mut dto = conn.query_row(
        "SELECT id, task_id, learner_id, artifact_uri, artifact_sha256,
                result_json, deviations, submitted_at_us
         FROM reproduction_submissions WHERE id = ?1",
        [submission_id],
        |row| {
            let result_json: Option<String> = row.get(5)?;
            Ok(ReproductionSubmissionDto {
                id: row.get(0)?,
                task_id: row.get(1)?,
                learner_id: row.get(2)?,
                artifact_uri: row.get(3)?,
                artifact_sha256: row.get(4)?,
                result: result_json
                    .as_deref()
                    .map(parse_json)
                    .transpose()?,
                deviations: row.get(6)?,
                submitted_at_us: row.get(7)?,
                evaluations: Vec::new(),
            })
        },
    )?;
    let evaluation_ids = query_ids(
        conn,
        "SELECT id FROM reproduction_evaluations WHERE submission_id = ?1 ORDER BY created_at_us, id",
        submission_id,
    )?;
    dto.evaluations = evaluation_ids
        .iter()
        .map(|evaluation_id| query_reproduction_evaluation(conn, evaluation_id))
        .collect::<Result<Vec<_>>>()?;
    Ok(dto)
}

fn query_reproduction_evaluation(
    conn: &Connection,
    evaluation_id: &str,
) -> Result<ReproductionEvaluationDto> {
    let mut dto = conn.query_row(
        "SELECT id, submission_id, rubric_version_id, evaluator_id, outcome, score,
                rationale, evidence_description, created_at_us
         FROM reproduction_evaluations WHERE id = ?1",
        [evaluation_id],
        |row| {
            let outcome: String = row.get(4)?;
            Ok(ReproductionEvaluationDto {
                id: row.get(0)?,
                submission_id: row.get(1)?,
                rubric_version_id: row.get(2)?,
                evaluator_id: row.get(3)?,
                outcome: parse_outcome(&outcome)?,
                score: row.get(5)?,
                rationale: row.get(6)?,
                evidence_description: row.get(7)?,
                criterion_scores: Vec::new(),
                created_at_us: row.get(8)?,
            })
        },
    )?;
    dto.criterion_scores = query_score_rows(
        conn,
        "SELECT rubric_criterion_id, score FROM reproduction_evaluation_scores
         WHERE evaluation_id = ?1 ORDER BY rubric_criterion_id",
        evaluation_id,
    )?;
    Ok(dto)
}

fn query_ids(conn: &Connection, sql: &str, parent_id: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement
        .query_map([parent_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn query_progress(conn: &Connection) -> Result<Vec<LearnerProgressDto>> {
    let objective_ids = {
        let mut statement =
            conn.prepare("SELECT id FROM learning_objective_versions ORDER BY learning_project_id, id")?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };
    let mut progress = Vec::new();
    for objective_id in objective_ids {
        let (project_id, learner_id): (String, String) = conn.query_row(
            "SELECT ov.learning_project_id, p.learner_id
             FROM learning_objective_versions ov
             JOIN learning_projects p ON p.id = ov.learning_project_id
             WHERE ov.id = ?1",
            [&objective_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let latest_evaluation: Option<(String, i64)> = conn
            .query_row(
                "SELECT e.id, e.created_at_us FROM attempt_evaluations e
                 JOIN assessment_attempts a ON a.id = e.attempt_id
                 JOIN assessment_version_objectives avo
                   ON avo.assessment_version_id = a.assessment_version_id
                 WHERE a.learner_id = ?1 AND a.learning_project_id = ?2
                   AND avo.objective_version_id = ?3
                   AND NOT EXISTS (
                       SELECT 1 FROM attempt_evaluations later
                       WHERE later.attempt_id = e.attempt_id
                         AND later.revision_no > e.revision_no
                   )
                 ORDER BY e.created_at_us DESC, e.id DESC LIMIT 1",
                params![learner_id, project_id, objective_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let latest_record: Option<(String, String, i64)> = conn
            .query_row(
                "SELECT cr.id, cr.outcome, cr.created_at_us FROM competency_records cr
                 JOIN competency_record_objectives cro ON cro.competency_record_id = cr.id
                 WHERE cr.learner_id = ?1 AND cr.learning_project_id = ?2
                   AND cro.objective_version_id = ?3
                   AND NOT EXISTS (
                       SELECT 1 FROM competency_records newer
                       WHERE newer.competency_id = cr.competency_id
                         AND newer.learner_id = cr.learner_id
                         AND newer.created_at_us > cr.created_at_us
                   )
                 ORDER BY cr.created_at_us DESC, cr.id DESC LIMIT 1",
                params![learner_id, project_id, objective_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let (status, explanation, competency_record_id) = match (&latest_evaluation, &latest_record) {
            (Some((_, evaluation_at)), Some((record_id, outcome, record_at)))
                if evaluation_at > record_at =>
            {
                (
                    "reassessment_required".to_string(),
                    "A newer evaluated attempt exists than the latest competency record; reevaluate the configured mastery rule."
                        .to_string(),
                    Some(record_id.clone()),
                )
            }
            (_, Some((record_id, outcome, _))) if outcome == "mastered" => (
                "mastered".to_string(),
                "The latest competency record reports mastery under its pinned rule version."
                    .to_string(),
                Some(record_id.clone()),
            ),
            (_, Some((record_id, _, _))) => (
                "not_mastered".to_string(),
                "The latest competency record reports unmet mastery requirements.".to_string(),
                Some(record_id.clone()),
            ),
            (Some((_evaluation_id, _)), None) => (
                "evaluated".to_string(),
                "An assessment evaluation exists, but no competency mastery record has been created."
                    .to_string(),
                None,
            ),
            (None, None) => (
                "not_started".to_string(),
                "No assessment evaluation or competency record exists for this objective."
                    .to_string(),
                None,
            ),
        };
        progress.push(LearnerProgressDto {
            learner_id,
            learning_project_id: project_id,
            objective_version_id: objective_id,
            status,
            explanation,
            latest_evaluation_id: latest_evaluation.map(|value| value.0),
            competency_record_id,
        });
    }
    Ok(progress)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn create_framework_fixture(conn: &Connection) {
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE knowledge_frameworks (
                 id TEXT PRIMARY KEY,
                 title TEXT NOT NULL
             );
             CREATE TABLE knowledge_framework_versions (
                 id TEXT PRIMARY KEY,
                 framework_id TEXT NOT NULL REFERENCES knowledge_frameworks(id),
                 title TEXT NOT NULL,
                 version_no INTEGER NOT NULL,
                 status TEXT NOT NULL,
                 published_at_us INTEGER,
                 UNIQUE (id, framework_id)
             );
             INSERT INTO knowledge_frameworks (id, title) VALUES ('framework-a', 'Framework A');
             INSERT INTO knowledge_framework_versions
                 (id, framework_id, title, version_no, status, published_at_us)
             VALUES
                 ('framework-a-v1', 'framework-a', 'Framework A', 1, 'published', 1),
                 ('framework-a-v2', 'framework-a', 'Framework A', 2, 'published', 2);",
        )
        .expect("create framework fixture");
    }

    fn connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open test database");
        create_framework_fixture(&conn);
        initialize_schema(&conn).expect("initialize Learning schema");
        conn
    }

    fn persistent_connection(path: &Path) -> Connection {
        let conn = Connection::open(path).expect("open persistent test database");
        create_framework_fixture(&conn);
        initialize_schema(&conn).expect("initialize persistent Learning schema");
        conn
    }

    fn create_learning_project(
        conn: &mut Connection,
        learner_id: &str,
        framework_version_id: &str,
    ) -> LearningProjectDto {
        create_project(
            conn,
            &CreateLearningProjectReq {
                title: format!("Project for {framework_version_id}"),
                learner_id: learner_id.to_string(),
                framework_id: "framework-a".to_string(),
                framework_version_id: framework_version_id.to_string(),
                created_by: None,
            },
        )
        .expect("create framework-pinned project")
    }

    #[test]
    fn learning_projects_pin_coexisting_framework_versions() {
        let path = std::env::temp_dir().join(format!("vora-learning-{}.sqlite", id()));
        let mut conn = persistent_connection(&path);
        initialize_schema(&conn).expect("repeat migration safely");
        let learner = create_learner(
            &mut conn,
            &CreateLearnerReq {
                display_name: "Learner".to_string(),
                created_by: None,
            },
        )
        .expect("create learner");

        let v1_project = create_learning_project(&mut conn, &learner.id, "framework-a-v1");
        let v2_project = create_learning_project(&mut conn, &learner.id, "framework-a-v2");
        assert_eq!(v1_project.framework_version_id, "framework-a-v1");
        assert_eq!(v2_project.framework_version_id, "framework-a-v2");
        let objective = create_objective_version(
            &mut conn,
            &CreateLearningObjectiveReq {
                objective_id: None,
                learning_project_id: v1_project.id.clone(),
                title: "Persisted objective".to_string(),
                observable_outcome: "The learner can explain it".to_string(),
                criteria_text: "Explanation is accurate".to_string(),
                level: "introductory".to_string(),
                parent_version_id: None,
                prerequisite_version_ids: Vec::new(),
                created_by: None,
            },
        )
        .expect("create persisted objective");

        let update = conn.execute(
            "UPDATE learning_projects SET framework_version_id = 'framework-a-v2'
             WHERE id = ?1",
            [&v1_project.id],
        );
        assert!(update.is_err(), "a project framework pin must be immutable");

        drop(conn);
        let reopened_conn = Connection::open(&path).expect("reopen persistent test database");
        initialize_schema(&reopened_conn).expect("verify migration after restart");
        let framework_version_id: String = reopened_conn
            .query_row(
                "SELECT framework_version_id FROM learning_projects WHERE id = ?1",
                [&v1_project.id],
                |row| row.get(0),
            )
            .expect("read persisted framework pin");
        assert_eq!(framework_version_id, "framework-a-v1");
        let project_count: i64 = reopened_conn
            .query_row("SELECT COUNT(*) FROM learning_projects", [], |row| row.get(0))
            .expect("read projects after restart");
        assert_eq!(project_count, 2);
        let persisted_workspace = workspace(&reopened_conn).expect("restore Learning workspace");
        assert!(persisted_workspace.objectives.iter().any(|item| item.id == objective.id));
        let progress = persisted_workspace
            .progress
            .iter()
            .find(|item| item.objective_version_id == objective.id)
            .expect("restore objective progress");
        assert_eq!(progress.status, "not_started");
        drop(reopened_conn);
        std::fs::remove_file(path).expect("remove persistent test database");
    }

    #[test]
    fn submitted_attempt_and_evaluation_corrections_preserve_history() {
        let mut conn = connection();
        let learner = create_learner(
            &mut conn,
            &CreateLearnerReq {
                display_name: "Learner".to_string(),
                created_by: None,
            },
        )
        .expect("create learner");
        let project = create_learning_project(&mut conn, &learner.id, "framework-a-v1");
        let objective = create_objective_version(
            &mut conn,
            &CreateLearningObjectiveReq {
                objective_id: None,
                learning_project_id: project.id.clone(),
                title: "Explain the concept".to_string(),
                observable_outcome: "Learner explains the concept".to_string(),
                criteria_text: "Explanation is accurate".to_string(),
                level: "introductory".to_string(),
                parent_version_id: None,
                prerequisite_version_ids: Vec::new(),
                created_by: None,
            },
        )
        .expect("create objective version");

        let assessment = create_assessment_version(
            &mut conn,
            &CreateAssessmentVersionReq {
                assessment_id: None,
                learning_project_id: project.id.clone(),
                title: "Concept check".to_string(),
                change_rationale: "Initial assessment version".to_string(),
                rubric_title: "Concept rubric".to_string(),
                rubric_pass_threshold: 0.7,
                questions: vec![CreateAssessmentQuestionReq {
                    objective_version_id: objective.id.clone(),
                    question_order: 0,
                    prompt: "Explain the concept".to_string(),
                    response_kind: "text".to_string(),
                    max_points: 1.0,
                    required: true,
                }],
                rubric_criteria: vec![CreateRubricCriterionReq {
                    objective_version_id: objective.id.clone(),
                    criterion_key: "accuracy".to_string(),
                    description: "Accurate explanation".to_string(),
                    max_points: 1.0,
                }],
                created_by: None,
            },
        )
        .expect("create assessment draft");
        review_assessment(
            &mut conn,
            &ReviewAssessmentReq {
                assessment_version_id: assessment.id.clone(),
                review_status: AssessmentReviewStatus::Approved,
                reviewer_id: Some("reviewer".to_string()),
                rationale: "Reviewed for use".to_string(),
            },
        )
        .expect("approve assessment");
        transition_assessment_version(
            &mut conn,
            &TransitionAssessmentVersionReq {
                assessment_version_id: assessment.id.clone(),
                next_status: LearningVersionStatus::Published,
                actor_id: Some("reviewer".to_string()),
            },
        )
        .expect("publish assessment");

        let task = create_reproduction_task(
            &mut conn,
            &CreateReproductionTaskReq {
                learning_project_id: project.id.clone(),
                objective_version_id: objective.id.clone(),
                rubric_version_id: assessment.rubric_version_id.clone(),
                title: "Reproduce the explanation".to_string(),
                task_description: "Provide an independent explanation.".to_string(),
                conditions: "Use the pinned framework version.".to_string(),
                expected_result: serde_json::json!({"key": "explanation"}),
                created_by: None,
            },
        )
        .expect("create reproduction task");
        assert!(
            submit_reproduction(
                &mut conn,
                &SubmitReproductionReq {
                    task_id: task.id.clone(),
                    learner_id: learner.id.clone(),
                    artifact_uri: None,
                    artifact_sha256: None,
                    result: None,
                    deviations: String::new(),
                },
            )
            .is_err(),
            "reproduction requires a result or checksum-pinned artifact"
        );
        let submission = submit_reproduction(
            &mut conn,
            &SubmitReproductionReq {
                task_id: task.id.clone(),
                learner_id: learner.id.clone(),
                artifact_uri: None,
                artifact_sha256: None,
                result: Some(serde_json::json!({"key": "learner response"})),
                deviations: "No deviations".to_string(),
            },
        )
        .expect("submit structured reproduction result");
        let reproduction_request = |evaluator_id: String| EvaluateReproductionReq {
            submission_id: submission.id.clone(),
            evaluator_id,
            outcome: AssessmentOutcome::Pass,
            criterion_scores: vec![EvaluationCriterionScoreDto {
                rubric_criterion_id: assessment.rubric_criteria[0].id.clone(),
                score: 1.0,
            }],
            rationale: "Meets the independent rubric.".to_string(),
            evidence_description: "Inspected the submitted structured result.".to_string(),
        };
        assert!(
            evaluate_reproduction(&mut conn, &reproduction_request(learner.id.clone())).is_err(),
            "the learner cannot evaluate their own reproduction"
        );
        let reproduction_evaluation =
            evaluate_reproduction(&mut conn, &reproduction_request("independent-evaluator".to_string()))
                .expect("evaluate independent reproduction");
        assert_eq!(reproduction_evaluation.outcome, AssessmentOutcome::Pass);

        let attempt = create_attempt(
            &mut conn,
            &CreateAssessmentAttemptReq {
                learning_project_id: project.id.clone(),
                learner_id: learner.id.clone(),
                assessment_version_id: assessment.id.clone(),
            },
        )
        .expect("create pinned attempt");
        assert_eq!(attempt.framework_version_id, "framework-a-v1");
        save_attempt_response(
            &conn,
            &SaveAttemptResponseReq {
                attempt_id: attempt.id.clone(),
                assessment_question_id: assessment.questions[0].id.clone(),
                response: serde_json::json!("A recorded answer"),
            },
        )
        .expect("save answer");
        submit_attempt(&mut conn, &attempt.id).expect("submit attempt");
        assert!(
            save_attempt_response(
                &conn,
                &SaveAttemptResponseReq {
                    attempt_id: attempt.id.clone(),
                    assessment_question_id: assessment.questions[0].id.clone(),
                    response: serde_json::json!("Changed answer"),
                },
            )
            .is_err(),
            "submitted responses cannot be rewritten"
        );

        let first = evaluate_attempt(
            &mut conn,
            &EvaluateAssessmentAttemptReq {
                attempt_id: attempt.id.clone(),
                correction_of_id: None,
                evaluator_id: "reviewer".to_string(),
                outcome: AssessmentOutcome::Pass,
                criterion_scores: vec![EvaluationCriterionScoreDto {
                    rubric_criterion_id: assessment.rubric_criteria[0].id.clone(),
                    score: 1.0,
                }],
                rationale: "Initial evaluation".to_string(),
                feedback: "Clear explanation".to_string(),
            },
        )
        .expect("evaluate submission");
        let corrected = evaluate_attempt(
            &mut conn,
            &EvaluateAssessmentAttemptReq {
                attempt_id: attempt.id.clone(),
                correction_of_id: Some(first.id.clone()),
                evaluator_id: "reviewer".to_string(),
                outcome: AssessmentOutcome::Fail,
                criterion_scores: vec![EvaluationCriterionScoreDto {
                    rubric_criterion_id: assessment.rubric_criteria[0].id.clone(),
                    score: 0.0,
                }],
                rationale: "Correction after evidence review".to_string(),
                feedback: "Correction retained as a new revision".to_string(),
            },
        )
        .expect("append evaluation correction");
        assert_eq!(corrected.revision_no, 2);
        assert_eq!(corrected.correction_of_id.as_deref(), Some(first.id.as_str()));

        let competency = create_competency(
            &mut conn,
            &CreateCompetencyReq {
                learning_project_id: project.id.clone(),
                title: "Concept mastery".to_string(),
                description: "Demonstrates the pinned learning objective".to_string(),
                criteria: vec![CreateCompetencyCriterionReq {
                    objective_version_id: objective.id,
                    criterion: "Accurate explanation".to_string(),
                    minimum_score: 0.7,
                    required: true,
                }],
                created_by: None,
            },
        )
        .expect("create competency");
        let mastery_rule = create_mastery_rule_version(
            &mut conn,
            &CreateMasteryRuleVersionReq {
                mastery_rule_id: None,
                competency_id: competency.id.clone(),
                title: "One passing evaluation".to_string(),
                minimum_average_score: 0.7,
                minimum_evaluated_attempts: 1,
                all_required_criteria: true,
                reproduction_required: false,
                required_reproduction_outcome: AssessmentOutcome::Pass,
                created_by: None,
            },
        )
        .expect("create explicit mastery rule");
        let rule_version = &mastery_rule.versions[0];
        let mastery_record = evaluate_competency(
            &mut conn,
            &EvaluateCompetencyReq {
                competency_id: competency.id,
                learner_id: learner.id,
                mastery_rule_version_id: rule_version.id.clone(),
            },
        )
        .expect("evaluate explicit mastery rule");
        assert_eq!(mastery_record.outcome, MasteryOutcome::NotMastered);
        assert_eq!(mastery_record.attempt_evaluation_ids, vec![corrected.id.clone()]);

        let stored = query_attempt(&conn, &attempt.id).expect("read attempt history");
        assert_eq!(stored.status, "submitted");
        assert_eq!(stored.framework_version_id, "framework-a-v1");
        assert_eq!(stored.evaluations.len(), 2);
        assert_eq!(stored.evaluations[0].id, first.id);
        assert_eq!(stored.evaluations[1].id, corrected.id);
    }
}

