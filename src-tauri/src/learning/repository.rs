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
        statement
            .query_map([&rubric_version_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<HashMap<_, _>>>()?
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

