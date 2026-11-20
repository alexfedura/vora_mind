use super::*;
use crate::mind::canvas_db::AppState;
use tauri::State;

#[tauri::command]
pub fn research_get_workspace(state: State<AppState>) -> Result<ResearchWorkspaceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    workspace(&conn)
}

#[tauri::command]
pub fn research_get_project_context(
    state: State<AppState>,
    project_id: String,
) -> Result<ResearchProjectContextDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    project_context(&conn, &project_id)
}

#[tauri::command]
pub fn research_create_project(
    state: State<AppState>,
    request: CreateResearchProjectReq,
) -> Result<ResearchProjectDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_project(&mut conn, request)
}

#[tauri::command]
pub fn research_transition_project(
    state: State<AppState>,
    project_id: String,
    next_status: ResearchProjectStatus,
    actor_id: Option<String>,
) -> Result<ResearchProjectDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    transition_project(&conn, &project_id, next_status, actor_id.as_deref())
}

#[tauri::command]
pub fn research_associate_project_source(
    state: State<AppState>,
    request: AssociateResearchProjectSourceReq,
) -> Result<ResearchProjectSourceDto, ResearchProjectSourceError> {
    let mut conn = state
        .db
        .lock()
        .map_err(|error| ResearchProjectSourceError::StorageError(error.to_string()))?;
    associate_project_source(&mut conn, request)
}

#[tauri::command]
pub fn research_list_project_sources(
    state: State<AppState>,
    project_id: String,
) -> Result<Vec<ResearchProjectSourceDto>, ResearchProjectSourceError> {
    let conn = state
        .db
        .lock()
        .map_err(|error| ResearchProjectSourceError::StorageError(error.to_string()))?;
    list_project_sources(&conn, &project_id)
}

#[tauri::command]
pub fn research_list_source_projects(
    state: State<AppState>,
    source_id: String,
) -> Result<Vec<ResearchSourceProjectSummaryDto>, ResearchProjectSourceError> {
    let conn = state
        .db
        .lock()
        .map_err(|error| ResearchProjectSourceError::StorageError(error.to_string()))?;
    list_source_projects(&conn, &source_id)
}

#[tauri::command]
pub fn research_get_project_source(
    state: State<AppState>,
    project_id: String,
    source_id: String,
) -> Result<ResearchProjectSourceDto, ResearchProjectSourceError> {
    let conn = state
        .db
        .lock()
        .map_err(|error| ResearchProjectSourceError::StorageError(error.to_string()))?;
    get_project_source(&conn, &project_id, &source_id)
}

#[tauri::command]
pub fn research_update_project_source_context(
    state: State<AppState>,
    request: UpdateResearchProjectSourceContextReq,
) -> Result<ResearchProjectSourceDto, ResearchProjectSourceError> {
    let mut conn = state
        .db
        .lock()
        .map_err(|error| ResearchProjectSourceError::StorageError(error.to_string()))?;
    update_project_source_context(&mut conn, request)
}

#[tauri::command]
pub fn research_create_domain(
    state: State<AppState>,
    request: CreateResearchDomainReq,
) -> Result<ResearchDomainDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_domain(&mut conn, request)
}

#[tauri::command]
pub fn research_assign_domain(
    state: State<AppState>,
    project_id: String,
    domain_id: String,
    rationale: String,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    assign_domain(&conn, &project_id, &domain_id, &rationale)
}

#[tauri::command]
pub fn research_adopt_framework_version(
    state: State<AppState>,
    project_id: String,
    framework_id: String,
    framework_version_id: String,
    use_context: String,
    actor_id: Option<String>,
) -> Result<ProjectKnowledgeAdoptionDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    adopt_framework_version(
        &conn,
        &project_id,
        &framework_id,
        &framework_version_id,
        &use_context,
        actor_id.as_deref(),
    )
}

#[tauri::command]
pub fn research_create_literature_review(
    state: State<AppState>,
    request: CreateLiteratureReviewReq,
) -> Result<LiteratureReviewDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_literature_review(&conn, request)
}

#[tauri::command]
pub fn research_create_reviewed_literature_review(
    state: State<AppState>,
    request: CreateReviewedLiteratureReviewReq,
) -> Result<LiteratureReviewDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_reviewed_literature_review(&conn, request)
}

#[tauri::command]
pub fn research_add_literature_source(
    state: State<AppState>,
    request: AddLiteratureSourceReq,
) -> Result<LiteratureReviewSourceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    add_literature_source(&conn, request)
}

#[tauri::command]
pub fn research_review_literature_review(
    state: State<AppState>,
    review_id: String,
    reviewer_id: Option<String>,
) -> Result<LiteratureReviewDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    review_literature_review(&conn, &review_id, reviewer_id.as_deref())
}

#[tauri::command]
pub fn research_create_prior_assessment(
    state: State<AppState>,
    request: CreatePriorKnowledgeAssessmentReq,
) -> Result<PriorKnowledgeAssessmentDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_prior_assessment(&conn, request)
}

#[tauri::command]
pub fn research_review_prior_assessment(
    state: State<AppState>,
    assessment_id: String,
    reviewer_id: Option<String>,
) -> Result<PriorKnowledgeAssessmentDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    review_prior_assessment(&conn, &assessment_id, reviewer_id.as_deref())
}

#[tauri::command]
pub fn research_create_gap(
    state: State<AppState>,
    request: CreateResearchGapReq,
) -> Result<ResearchGapDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_gap(&conn, request)
}

#[tauri::command]
pub fn research_add_gap_evidence(
    state: State<AppState>,
    request: AddGapEvidenceReq,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    add_gap_evidence(&conn, request)
}

#[tauri::command]
pub fn research_review_gap_exception(
    state: State<AppState>,
    request: ReviewGapExceptionReq,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    review_gap_exception(&conn, request)
}

#[tauri::command]
pub fn research_transition_gap(
    state: State<AppState>,
    gap_id: String,
    next_status: ResearchGapStatus,
    actor_id: Option<String>,
) -> Result<ResearchGapDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    transition_gap(&conn, &gap_id, next_status, actor_id.as_deref())
}

#[tauri::command]
pub fn research_create_question(
    state: State<AppState>,
    request: CreateResearchQuestionReq,
) -> Result<ResearchQuestionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_question(&mut conn, request)
}

#[tauri::command]
pub fn research_assign_unassigned_question(
    state: State<AppState>,
    question_id: String,
    project_id: String,
    rationale: String,
    actor_id: Option<String>,
) -> Result<ResearchQuestionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    assign_unassigned_question(
        &mut conn,
        &question_id,
        &project_id,
        &rationale,
        actor_id.as_deref(),
    )
}

#[tauri::command]
pub fn research_transition_question(
    state: State<AppState>,
    question_id: String,
    next_status: ResearchQuestionStatus,
    actor_id: Option<String>,
    rationale: String,
) -> Result<ResearchQuestionDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    transition_question(
        &conn,
        &question_id,
        next_status,
        actor_id.as_deref(),
        &rationale,
    )
}

#[tauri::command]
pub fn research_refine_question(
    state: State<AppState>,
    parent_question_id: String,
    title: String,
    description: String,
    rationale: String,
    actor_id: Option<String>,
) -> Result<ResearchQuestionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    refine_question(
        &mut conn,
        &parent_question_id,
        &title,
        &description,
        &rationale,
        actor_id.as_deref(),
    )
}

#[tauri::command]
pub fn research_link_question_lineage(
    state: State<AppState>,
    parent_question_id: String,
    derived_question_id: String,
    relation_kind: ResearchQuestionRelation,
    rationale: String,
    actor_id: Option<String>,
) -> Result<ResearchQuestionLineageDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    link_question_lineage(
        &conn,
        &parent_question_id,
        &derived_question_id,
        relation_kind,
        &rationale,
        actor_id.as_deref(),
    )
}

#[tauri::command]
pub fn research_create_phenomenon(
    state: State<AppState>,
    request: CreatePhenomenonReq,
) -> Result<PhenomenonDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_phenomenon(&conn, request)
}

#[tauri::command]
pub fn research_create_observation_plan(
    state: State<AppState>,
    request: CreateObservationPlanReq,
) -> Result<ObservationPlanDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_observation_plan(&mut conn, request)
}

#[tauri::command]
pub fn research_review_observation_plan(
    state: State<AppState>,
    plan_version_id: String,
    preregister: bool,
    actor_id: Option<String>,
) -> Result<ObservationPlanVersionDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    review_observation_plan(&conn, &plan_version_id, preregister, actor_id.as_deref())
}

#[tauri::command]
pub fn research_create_dataset_version_reference(
    state: State<AppState>,
    request: CreateDatasetVersionReferenceReq,
) -> Result<DatasetVersionReferenceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_dataset_version_reference(&conn, request)
}

#[tauri::command]
pub fn research_create_observation(
    state: State<AppState>,
    request: CreateObservationReq,
) -> Result<ObservationDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_observation(&conn, request)
}

#[tauri::command]
pub fn research_create_pattern(
    state: State<AppState>,
    request: CreatePatternReq,
) -> Result<PatternDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_pattern(&mut conn, request)
}

#[tauri::command]
pub fn research_create_idea(
    state: State<AppState>,
    request: CreateIdeaReq,
) -> Result<IdeaDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    create_idea(&conn, request)
}

#[tauri::command]
pub fn research_create_typed_hypothesis(
    state: State<AppState>,
    request: CreateTypedHypothesisReq,
) -> Result<ResearchHypothesisDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_typed_hypothesis(&mut conn, request)
}

#[tauri::command]
pub fn research_assign_unassigned_hypothesis(
    state: State<AppState>,
    hypothesis_id: String,
    project_id: String,
    rationale: String,
    actor_id: Option<String>,
) -> Result<ResearchHypothesisDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    assign_unassigned_hypothesis(
        &mut conn,
        &hypothesis_id,
        &project_id,
        &rationale,
        actor_id.as_deref(),
    )
}

#[tauri::command]
pub fn research_transition_hypothesis(
    state: State<AppState>,
    hypothesis_id: String,
    next_status: ResearchHypothesisStatus,
    actor_id: Option<String>,
) -> Result<ResearchHypothesisDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    transition_hypothesis(&conn, &hypothesis_id, next_status, actor_id.as_deref())
}

#[tauri::command]
pub fn research_link_competing_hypotheses(
    state: State<AppState>,
    request: LinkCompetingHypothesesReq,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    link_competing_hypotheses(&conn, request)
}

#[tauri::command]
pub fn research_add_hypothesis_model_reference(
    state: State<AppState>,
    request: AddHypothesisModelReferenceReq,
) -> Result<HypothesisModelReferenceDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    add_hypothesis_model_reference(&conn, request)
}

#[tauri::command]
pub fn research_add_hypothesis_evidence(
    state: State<AppState>,
    request: AddHypothesisEvidenceReq,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    add_hypothesis_evidence(&conn, request)
}

#[tauri::command]
pub fn research_create_design(
    state: State<AppState>,
    request: CreateResearchDesignReq,
) -> Result<ResearchExperimentDesignVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_research_design(&mut conn, request)
}

#[tauri::command]
pub fn research_create_linked_design(
    state: State<AppState>,
    request: CreateLinkedResearchDesignReq,
) -> Result<ResearchExperimentDesignVersionDto, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    create_linked_research_design(&mut conn, request)
}

#[tauri::command]
pub fn research_transition_design(
    state: State<AppState>,
    design_version_id: String,
    next_status: ResearchDesignStatus,
    actor_id: Option<String>,
) -> Result<ResearchExperimentDesignVersionDto, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    transition_research_design(&conn, &design_version_id, next_status, actor_id.as_deref())
}

#[tauri::command]
pub fn research_link_hypothesis_design(
    state: State<AppState>,
    request: LinkHypothesisDesignReq,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    link_hypothesis_design(&conn, request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mind::{canvas_db::AppState, portable_db::initialize_workspace_schema},
        source::{
            acquisition::{
                self, CreateKnowledgeExtractionReq, CreateSourceClaimReq, CreateSourceLocationReq,
                CreateSourceVersionReq, ExtractionKind, ExtractionReviewStatus,
                ReviewExtractionReq, SourceCaptureKind, SourceLocationKind,
            },
            ipc::{create_source_impl, CreateSourceReq},
        },
    };
    use rusqlite::Connection;
    use serde::de::DeserializeOwned;
    use serde_json::{json, Value};
    use std::{fs, path::PathBuf, sync::Mutex};
    use tauri::{
        ipc::{CallbackFn, InvokeBody},
        test::{
            get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY,
        },
        webview::InvokeRequest,
        Manager, WebviewWindow, WebviewWindowBuilder,
    };

    struct IsolatedDatabaseFile {
        path: PathBuf,
    }

    impl IsolatedDatabaseFile {
        fn new() -> Self {
            Self {
                path: std::env::temp_dir()
                    .join(format!("vora-research-ipc-{}.sqlite", uuid::Uuid::now_v7())),
            }
        }
    }

    impl Drop for IsolatedDatabaseFile {
        fn drop(&mut self) {
            for suffix in ["", "-journal", "-wal", "-shm"] {
                let path = PathBuf::from(format!("{}{suffix}", self.path.display()));
                if let Err(error) = fs::remove_file(&path) {
                    if error.kind() != std::io::ErrorKind::NotFound {
                        eprintln!("Could not remove isolated Research test file {path:?}: {error}");
                    }
                }
            }
        }
    }

    fn test_app() -> (
        IsolatedDatabaseFile,
        tauri::App<MockRuntime>,
        WebviewWindow<MockRuntime>,
    ) {
        let database_file = IsolatedDatabaseFile::new();
        let conn = Connection::open(&database_file.path).expect("open isolated database");
        initialize_workspace_schema(&conn).expect("initialize isolated workspace schema");
        conn.pragma_update(None, "foreign_keys", "ON")
            .expect("enable isolated foreign keys");
        let app = mock_builder()
            .manage(AppState {
                db: Mutex::new(conn),
                db_path: Mutex::new(Some(database_file.path.to_string_lossy().into_owned())),
            })
            .invoke_handler(tauri::generate_handler![
                research_get_workspace,
                research_get_project_context,
                research_create_project,
                research_transition_project,
                research_associate_project_source,
                research_create_reviewed_literature_review,
                research_create_prior_assessment,
                research_review_prior_assessment,
                research_create_gap,
                research_add_gap_evidence,
                research_transition_gap,
                research_create_question,
                research_transition_question,
                research_create_phenomenon,
                research_create_observation_plan,
                research_review_observation_plan,
                research_create_dataset_version_reference,
                research_create_observation,
                research_create_pattern,
                research_create_idea,
                research_create_typed_hypothesis,
                research_add_hypothesis_evidence,
                research_create_linked_design,
            ])
            .build(mock_context(noop_assets()))
            .expect("build isolated Tauri test app");
        let window = WebviewWindowBuilder::new(&app, "test", Default::default())
            .build()
            .expect("build mock webview");
        (database_file, app, window)
    }

    fn invoke<T: DeserializeOwned>(
        window: &WebviewWindow<MockRuntime>,
        command: &str,
        payload: Value,
    ) -> Result<T, Value> {
        let request = InvokeRequest {
            cmd: command.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: if cfg!(any(windows, target_os = "android")) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .expect("parse mock origin"),
            body: InvokeBody::Json(payload),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        };
        get_ipc_response(window, request)
            .map(|body| body.deserialize::<T>().expect("deserialize IPC response"))
    }

    fn error_text(error: Value) -> String {
        error
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| error.to_string())
    }

    fn create_active_project(
        window: &WebviewWindow<MockRuntime>,
        title: &str,
    ) -> ResearchProjectDto {
        let project = invoke::<ResearchProjectDto>(
            window,
            "research_create_project",
            json!({
                "request": {
                    "title": title,
                    "aim": "Test the Existing-Knowledge Investigation IPC path.",
                    "pathways": ["existing_knowledge_investigation"],
                    "createdBy": "phase-8.2-test"
                }
            }),
        )
        .expect("create project through IPC");
        invoke::<ResearchProjectDto>(
            window,
            "research_transition_project",
            json!({
                "projectId": project.id,
                "nextStatus": "active",
                "actorId": "phase-8.2-test"
            }),
        )
        .expect("activate project through IPC")
    }

    fn create_source_fixture(
        app: &tauri::App<MockRuntime>,
        label: &str,
    ) -> (String, String, String, String) {
        let state = app.state::<AppState>();
        let conn = state.db.lock().expect("lock isolated test database");
        let source_id = create_source_impl(
            &conn,
            CreateSourceReq {
                kind: "book".into(),
                title: format!("Synthetic IPC source {label}"),
                path: Some(format!("https://example.test/phase-8-2/{label}")),
                author: Some("Synthetic Author".into()),
                language: Some("en".into()),
                duration_sec: None,
                page_count: Some(5),
                notes: Some("Synthetic test data only.".into()),
            },
        )
        .expect("create synthetic source");
        let version = acquisition::create_source_version(
            &conn,
            CreateSourceVersionReq {
                source_id: source_id.clone(),
                capture_kind: SourceCaptureKind::Edition,
                edition_label: Some(format!("Synthetic edition {label}")),
                canonical_uri: Some(format!("https://example.test/phase-8-2/{label}")),
                captured_at_us: Some(1_000),
                content_sha256: None,
                external_asset_ref: None,
                metadata_json: Some("{}".into()),
            },
            Some("phase-8.2-test"),
        )
        .expect("create exact SourceVersion");
        let location = acquisition::create_source_location(
            &conn,
            CreateSourceLocationReq {
                source_id: source_id.clone(),
                source_version_id: version.id.clone(),
                legacy_segment_id: None,
                label: format!("Page 2, paragraph 1 {label}"),
                location_kind: SourceLocationKind::Paragraph,
                page_start: Some(2),
                page_end: Some(2),
                paragraph_start: Some(1),
                paragraph_end: Some(1),
                line_start: None,
                line_end: None,
                time_start_ms: None,
                time_end_ms: None,
                stable_selector: Some(format!("phase-8-2:{label}:p2:paragraph1")),
                locator_text: Some("Attributed synthetic statement.".into()),
                notes: None,
                parent_segment_id: None,
                order_index: Some(0),
            },
        )
        .expect("create exact SourceLocation");
        let extraction = acquisition::create_extraction(
            &conn,
            CreateKnowledgeExtractionReq {
                source_id: source_id.clone(),
                source_version_id: version.id.clone(),
                source_segment_id: location.id.clone(),
                kind: ExtractionKind::SourceClaim,
                content: "The synthetic source attributes a statement.".into(),
                language: Some("en".into()),
                parent_extraction_id: None,
                tool_identity: None,
                tool_version: None,
                prompt_config_identity: None,
                confidence_decimal: None,
                uncertainty: None,
                created_by: Some("phase-8.2-test".into()),
            },
        )
        .expect("create synthetic extraction");
        acquisition::review_extraction(
            &conn,
            ReviewExtractionReq {
                extraction_id: extraction.id.clone(),
                review_status: ExtractionReviewStatus::Accepted,
                reviewer_id: Some("phase-8.2-reviewer".into()),
                rationale: Some("Synthetic extraction matches its source.".into()),
            },
        )
        .expect("review synthetic extraction");
        let claim = acquisition::create_source_claim(
            &conn,
            CreateSourceClaimReq {
                extraction_id: extraction.id,
                statement: "The synthetic source attributes a statement.".into(),
                asserted_by: Some("Synthetic Author".into()),
                scope: Some("Attributed evidence, not a scientific conclusion.".into()),
            },
        )
        .expect("create attributed SourceClaim");
        (source_id, version.id, location.id, claim.id)
    }

    #[test]
    fn discovery_project_context_crosses_ipc_with_exact_scoped_collections() {
        let (database_file, app, window) = test_app();
        let project_a = invoke::<ResearchProjectDto>(
            &window,
            "research_create_project",
            json!({
                "request": {
                    "title": "Synthetic first-principles project",
                    "aim": "Trace a problem from observation to typed hypothesis.",
                    "pathways": ["first_principles_discovery"],
                    "createdBy": "phase-8.4.1-test"
                }
            }),
        )
        .expect("create Discovery project through IPC");
        invoke::<ResearchProjectDto>(
            &window,
            "research_transition_project",
            json!({
                "projectId": project_a.id,
                "nextStatus": "active",
                "actorId": "phase-8.4.1-test"
            }),
        )
        .expect("activate Discovery project");
        let project_b = create_active_project(&window, "Synthetic unrelated Discovery project");

        let question = invoke::<ResearchQuestionDto>(
            &window,
            "research_create_question",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "researchGapId": null,
                    "domainId": null,
                    "title": "Does temperature change the measured response?",
                    "description": "A synthetic first-principles question.",
                    "rationale": "The exact question is the lifecycle origin.",
                    "createdBy": "phase-8.4.1-test"
                }
            }),
        )
        .expect("create Discovery question");
        for status in ["review", "accepted"] {
            invoke::<ResearchQuestionDto>(
                &window,
                "research_transition_question",
                json!({
                    "questionId": question.id,
                    "nextStatus": status,
                    "actorId": "phase-8.4.1-test",
                    "rationale": "Synthetic accepted lifecycle transition."
                }),
            )
            .expect("transition Discovery question");
        }

        let phenomenon = invoke::<PhenomenonDto>(
            &window,
            "research_create_phenomenon",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "domainId": null,
                    "title": "Synthetic temperature response",
                    "description": "A measured response to temperature.",
                    "populationScope": "Synthetic samples",
                    "context": "Controlled synthetic conditions",
                    "temporalBoundary": "One synthetic session",
                    "createdBy": "phase-8.4.1-test"
                }
            }),
        )
        .expect("create phenomenon");

        let plan = invoke::<ObservationPlanDto>(
            &window,
            "research_create_observation_plan",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "phenomenonId": phenomenon.id,
                    "title": "Temperature observation plan",
                    "measurementMethod": "Synthetic sensor",
                    "sampleFrame": "Five labeled samples",
                    "conditionsTemplate": "Same instrument and environment",
                    "inclusionRules": "Only complete readings",
                    "createdBy": "phase-8.4.1-test"
                }
            }),
        )
        .expect("create observation plan");
        let plan_version = invoke::<ObservationPlanVersionDto>(
            &window,
            "research_review_observation_plan",
            json!({
                "planVersionId": plan.versions[0].id,
                "preregister": true,
                "actorId": "phase-8.4.1-test"
            }),
        )
        .expect("preregister observation plan");

        let dataset = invoke::<DatasetVersionReferenceDto>(
            &window,
            "research_create_dataset_version_reference",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "title": "Synthetic temperature dataset",
                    "versionLabel": "v1",
                    "externalUri": "synthetic://phase-8-4-1/temperature-dataset/v1",
                    "contentSha256": null,
                    "sourceVersionId": null,
                    "createdBy": "phase-8.4.1-test"
                }
            }),
        )
        .expect("create dataset reference");

        let observation = invoke::<ObservationDto>(
            &window,
            "research_create_observation",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "phenomenonId": phenomenon.id,
                    "planId": plan.id,
                    "planVersionId": plan_version.id,
                    "datasetVersionReferenceId": dataset.id,
                    "measuredAtUs": 1_700_000_000_000_000_i64,
                    "valueDecimal": "4.2",
                    "valueState": "observed",
                    "unit": "points",
                    "measurementMethod": "Synthetic sensor",
                    "conditions": "Controlled synthetic environment",
                    "sampleIdentifier": "sample-1",
                    "createdBy": "phase-8.4.1-test"
                }
            }),
        )
        .expect("create observation");

        let pattern = invoke::<PatternDto>(
            &window,
            "research_create_pattern",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "title": "Synthetic increasing response",
                    "statement": "The synthetic observations increase across the declared samples.",
                    "analysisMethod": "Ordered synthetic comparison",
                    "uncertainty": "One-sample fixture; uncertainty is unresolved.",
                    "observationIds": [observation.id],
                    "datasetVersionReferenceIds": [dataset.id],
                    "createdBy": "phase-8.4.1-test"
                }
            }),
        )
        .expect("create pattern");

        let idea = invoke::<IdeaDto>(
            &window,
            "research_create_idea",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "title": "Temperature mechanism idea",
                    "content": "An unvalidated proposal linked to the observed pattern.",
                    "phenomenonId": null,
                    "observationId": null,
                    "patternId": pattern.id,
                    "questionId": null,
                    "createdBy": "phase-8.4.1-test"
                }
            }),
        )
        .expect("create idea");

        let hypothesis = invoke::<ResearchHypothesisDto>(
            &window,
            "research_create_typed_hypothesis",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "researchQuestionId": question.id,
                    "parentHypothesisId": null,
                    "lineageRationale": null,
                    "statementH0": "Temperature has no effect on the response.",
                    "statementH1": "Temperature changes the measured response.",
                    "populationScope": "Synthetic samples",
                    "researchScope": "The synthetic controlled session",
                    "expectedDirection": "non_directional",
                    "expectedEffectDecimal": null,
                    "expectedEffectUnit": null,
                    "falsificationCriteria": "Reject H1 if no declared change is observed.",
                    "assumptions": "Synthetic readings are comparable.",
                    "variables": [
                        {
                            "role": "independent",
                            "name": "Temperature",
                            "operationalDefinition": "Instrument setting in degrees Celsius",
                            "valueKind": "continuous",
                            "unit": "C",
                            "measurementMethod": "Synthetic sensor"
                        },
                        {
                            "role": "dependent",
                            "name": "Response",
                            "operationalDefinition": "Response score",
                            "valueKind": "continuous",
                            "unit": "points",
                            "measurementMethod": "Synthetic scale"
                        }
                    ],
                    "createdBy": "phase-8.4.1-test"
                }
            }),
        )
        .expect("create typed hypothesis");
        let definition = hypothesis
            .definitions
            .first()
            .expect("typed hypothesis definition");
        let design = invoke::<ResearchExperimentDesignVersionDto>(
            &window,
            "research_create_linked_design",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "title": "Synthetic first-principles design",
                    "description": "Design-only linkage for the typed hypothesis.",
                    "protocol": "No execution occurs in this synthetic fixture.",
                    "hypothesisVersionId": definition.id,
                    "relationshipKind": "tests",
                    "rationale": "Explicit design-to-hypothesis version linkage.",
                    "createdBy": "phase-8.4.1-test"
                }
            }),
        )
        .expect("create linked design");

        invoke::<()>(
            &window,
            "research_add_hypothesis_evidence",
            json!({
                "request": {
                    "hypothesisVersionId": definition.id,
                    "sourceClaimId": null,
                    "observationId": observation.id,
                    "patternId": null,
                    "polarity": "supports",
                    "rationale": "An exact synthetic observation evidence link.",
                    "createdBy": "phase-8.4.1-test"
                }
            }),
        )
        .expect("attach exact observation evidence through IPC");

        let cross_project_observation = invoke::<ObservationDto>(
            &window,
            "research_create_observation",
            json!({
                "request": {
                    "projectId": project_b.id,
                    "phenomenonId": phenomenon.id,
                    "planId": plan.id,
                    "planVersionId": plan_version.id,
                    "datasetVersionReferenceId": dataset.id,
                    "measuredAtUs": 1_700_000_000_000_001_i64,
                    "valueDecimal": "9.9",
                    "valueState": "observed",
                    "unit": "points",
                    "measurementMethod": "Synthetic sensor",
                    "conditions": "Must be rejected",
                    "sampleIdentifier": "wrong-project",
                    "createdBy": "phase-8.4.1-test"
                }
            }),
        )
        .expect_err("cross-project observation references must fail");
        assert!(!error_text(cross_project_observation).is_empty());

        let missing_project = invoke::<ResearchProjectContextDto>(
            &window,
            "research_get_project_context",
            json!({ "projectId": "missing-phase-8-4-1-project" }),
        )
        .expect_err("missing project context must fail");
        assert!(error_text(missing_project).contains("was not found"));

        let context_a = invoke::<ResearchProjectContextDto>(
            &window,
            "research_get_project_context",
            json!({ "projectId": project_a.id }),
        )
        .expect("retrieve Discovery project context");
        assert_eq!(context_a.project.id, project_a.id);
        assert_eq!(context_a.questions[0].id, question.id);
        assert_eq!(context_a.phenomena[0].id, phenomenon.id);
        assert_eq!(context_a.observation_plans[0].id, plan.id);
        assert_eq!(
            context_a.observation_plans[0].versions[0].id,
            plan_version.id
        );
        assert_eq!(
            context_a.observation_plans[0].versions[0].lifecycle_status,
            ObservationPlanStatus::Preregistered
        );
        assert_eq!(context_a.dataset_versions[0].id, dataset.id);
        assert_eq!(context_a.observations[0].id, observation.id);
        assert_eq!(context_a.observations[0].phenomenon_id, phenomenon.id);
        assert_eq!(context_a.observations[0].plan_id, plan.id);
        assert_eq!(context_a.observations[0].plan_version_id, plan_version.id);
        assert_eq!(
            context_a.observations[0]
                .dataset_version_reference_id
                .as_deref(),
            Some(dataset.id.as_str())
        );
        assert_eq!(context_a.patterns[0].id, pattern.id);
        assert_eq!(
            context_a.patterns[0].observation_ids,
            vec![observation.id.clone()]
        );
        assert_eq!(
            context_a.patterns[0].dataset_version_reference_ids,
            vec![dataset.id.clone()]
        );
        assert_eq!(context_a.ideas[0].id, idea.id);
        assert_eq!(
            context_a.ideas[0].pattern_id.as_deref(),
            Some(pattern.id.as_str())
        );
        assert_eq!(context_a.hypotheses[0].id, hypothesis.id);
        assert_eq!(
            context_a.hypotheses[0].research_question_id.as_deref(),
            Some(question.id.as_str())
        );
        assert_eq!(context_a.hypotheses[0].evidence.len(), 1);
        assert_eq!(
            context_a.hypotheses[0].evidence[0].target_kind,
            "observation"
        );
        assert_eq!(
            context_a.hypotheses[0].evidence[0].target_id,
            observation.id
        );
        assert_eq!(
            context_a.hypotheses[0].evidence[0].polarity,
            ResearchPolarity::Supports
        );
        assert_eq!(
            context_a.hypotheses[0].evidence[0].rationale,
            "An exact synthetic observation evidence link."
        );
        assert_eq!(context_a.design_versions[0].id, design.id);
        assert_eq!(context_a.design_versions[0].project_id, project_a.id);

        let context_b = invoke::<ResearchProjectContextDto>(
            &window,
            "research_get_project_context",
            json!({ "projectId": project_b.id }),
        )
        .expect("retrieve unrelated project context");
        assert!(context_b.phenomena.is_empty());
        assert!(context_b.observation_plans.is_empty());
        assert!(context_b.dataset_versions.is_empty());
        assert!(context_b.observations.is_empty());
        assert!(context_b.patterns.is_empty());
        assert!(context_b.ideas.is_empty());
        assert!(context_b.hypotheses.is_empty());
        assert!(context_b.design_versions.is_empty());

        {
            let state = app.state::<AppState>();
            let mut conn = state.db.lock().expect("lock isolated test database");
            let placeholder = Connection::open_in_memory()
                .expect("open temporary connection for Discovery reopen");
            let previous = std::mem::replace(&mut *conn, placeholder);
            previous.close().expect("close isolated Discovery database");
            let reopened =
                Connection::open(&database_file.path).expect("reopen isolated Discovery database");
            reopened
                .pragma_update(None, "foreign_keys", "ON")
                .expect("enable reopened foreign keys");
            *conn = reopened;
        }
        let reopened = invoke::<ResearchProjectContextDto>(
            &window,
            "research_get_project_context",
            json!({ "projectId": project_a.id }),
        )
        .expect("reload Discovery context after reopen");
        assert_eq!(reopened.phenomena[0].id, phenomenon.id);
        assert_eq!(reopened.observations[0].id, observation.id);
        assert_eq!(reopened.patterns[0].observation_ids, vec![observation.id]);
        assert_eq!(
            reopened.hypotheses[0].research_question_id.as_deref(),
            Some(question.id.as_str())
        );
        assert_eq!(reopened.design_versions[0].id, design.id);
    }

    #[test]
    fn existing_knowledge_workflow_crosses_ipc_with_project_and_exact_provenance() {
        let (database_file, app, window) = test_app();
        let project_a = create_active_project(&window, "Synthetic project A");
        let project_b = create_active_project(&window, "Synthetic project B");
        let (source_id, version_id, location_id, claim_id) = create_source_fixture(&app, "a");
        let (other_source_id, other_version_id, _, _) = create_source_fixture(&app, "b");

        invoke::<ResearchProjectSourceDto>(
            &window,
            "research_associate_project_source",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "sourceId": source_id,
                    "useContext": "Existing knowledge under investigation.",
                    "notes": "Association is not an exact citation."
                }
            }),
        )
        .expect("associate shared Source through IPC");
        let review = invoke::<LiteratureReviewDto>(
            &window,
            "research_create_reviewed_literature_review",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "title": "Synthetic exact-version review",
                    "searchScope": "One synthetic SourceVersion.",
                    "sourceId": source_id,
                    "sourceVersionId": version_id,
                    "sourceLocationId": location_id,
                    "sourceClaimId": claim_id,
                    "citationNote": "Exact synthetic provenance.",
                    "createdBy": "phase-8.2-test",
                    "reviewerId": "phase-8.2-reviewer"
                }
            }),
        )
        .expect("create reviewed citation through IPC");
        let version_only_review = invoke::<LiteratureReviewDto>(
            &window,
            "research_create_reviewed_literature_review",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "title": "Synthetic version-only review",
                    "searchScope": "The exact version without a location or claim.",
                    "sourceId": source_id,
                    "sourceVersionId": version_id,
                    "sourceLocationId": null,
                    "sourceClaimId": null,
                    "citationNote": "Partial provenance is explicit.",
                    "createdBy": "phase-8.2-test",
                    "reviewerId": "phase-8.2-reviewer"
                }
            }),
        )
        .expect("create exact-version citation without optional provenance");
        let assessment = invoke::<PriorKnowledgeAssessmentDto>(
            &window,
            "research_create_prior_assessment",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "title": "Synthetic prior-knowledge assessment",
                    "assessment": "A project-scoped assessment for the IPC fixture.",
                    "frameworkVersionId": null,
                    "createdBy": "phase-8.2-test"
                }
            }),
        )
        .expect("create project-scoped prior assessment through IPC");
        let assessment = invoke::<PriorKnowledgeAssessmentDto>(
            &window,
            "research_review_prior_assessment",
            json!({
                "assessmentId": assessment.id,
                "reviewerId": "phase-8.2-reviewer"
            }),
        )
        .expect("review prior assessment through IPC");
        assert_eq!(assessment.lifecycle_status, ReviewLifecycleStatus::Reviewed);

        let gap_a = invoke::<ResearchGapDto>(
            &window,
            "research_create_gap",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "domainId": null,
                    "title": "Project A gap",
                    "statement": "A test gap based on reviewed evidence.",
                    "createdBy": "phase-8.2-test"
                }
            }),
        )
        .expect("create Project A gap through IPC");
        invoke::<()>(
            &window,
            "research_add_gap_evidence",
            json!({
                "request": {
                    "gapId": gap_a.id,
                    "literatureReviewId": review.id,
                    "priorAssessmentId": null
                }
            }),
        )
        .expect("link reviewed evidence through IPC");
        invoke::<()>(
            &window,
            "research_add_gap_evidence",
            json!({
                "request": {
                    "gapId": gap_a.id,
                    "literatureReviewId": null,
                    "priorAssessmentId": assessment.id
                }
            }),
        )
        .expect("link prior assessment through IPC");
        for status in ["assessed", "accepted"] {
            invoke::<ResearchGapDto>(
                &window,
                "research_transition_gap",
                json!({
                    "gapId": gap_a.id,
                    "nextStatus": status,
                    "actorId": "phase-8.2-reviewer"
                }),
            )
            .expect("transition evidence-backed gap through IPC");
        }
        let question = invoke::<ResearchQuestionDto>(
            &window,
            "research_create_question",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "researchGapId": gap_a.id,
                    "domainId": null,
                    "title": "Project A question",
                    "description": "Linked to the accepted gap.",
                    "rationale": "Explicit project and gap lineage.",
                    "createdBy": "phase-8.2-test"
                }
            }),
        )
        .expect("create linked question through IPC");
        let gap_b = invoke::<ResearchGapDto>(
            &window,
            "research_create_gap",
            json!({
                "request": {
                    "projectId": project_b.id,
                    "domainId": null,
                    "title": "Project B gap",
                    "statement": "Control project record.",
                    "createdBy": "phase-8.2-test"
                }
            }),
        )
        .expect("create Project B control gap through IPC");
        let unassigned = invoke::<ResearchQuestionDto>(
            &window,
            "research_create_question",
            json!({
                "request": {
                    "projectId": null,
                    "researchGapId": null,
                    "domainId": null,
                    "title": "Unassigned control question",
                    "description": "Must remain unassigned.",
                    "rationale": "No project assignment requested.",
                    "createdBy": "phase-8.2-test"
                }
            }),
        )
        .expect("create unassigned control through IPC");

        let context_a = invoke::<ResearchProjectContextDto>(
            &window,
            "research_get_project_context",
            json!({ "projectId": project_a.id }),
        )
        .expect("retrieve Project A context through IPC");
        let context_b = invoke::<ResearchProjectContextDto>(
            &window,
            "research_get_project_context",
            json!({ "projectId": project_b.id }),
        )
        .expect("retrieve Project B context through IPC");

        assert_eq!(context_a.project_id, project_a.id);
        assert_eq!(context_a.project.id, project_a.id);
        assert_eq!(context_a.prior_assessments.len(), 1);
        assert_eq!(context_a.prior_assessments[0].id, assessment.id);
        assert_eq!(context_a.project_sources.len(), 1);
        assert_eq!(context_a.project_sources[0].source_id, source_id);
        assert_eq!(context_a.literature_reviews.len(), 2);
        let exact_review = context_a
            .literature_reviews
            .iter()
            .find(|item| item.id == review.id)
            .expect("retrieve exact-provenance review");
        assert_eq!(exact_review.project_id, project_a.id);
        assert_eq!(exact_review.sources[0].source_id, source_id);
        assert_eq!(exact_review.sources[0].source_version_id, version_id);
        assert_eq!(
            exact_review.sources[0].source_location_id.as_deref(),
            Some(location_id.as_str())
        );
        assert_eq!(
            exact_review.sources[0].source_claim_id.as_deref(),
            Some(claim_id.as_str())
        );
        let version_only = context_a
            .literature_reviews
            .iter()
            .find(|item| item.id == version_only_review.id)
            .expect("retrieve version-only review");
        assert_eq!(version_only.sources[0].source_version_id, version_id);
        assert_eq!(version_only.sources[0].source_location_id, None);
        assert_eq!(version_only.sources[0].source_claim_id, None);
        assert!(context_a.source_versions.iter().any(|reference| {
            reference.source_id == source_id
                && reference.source_version_id == version_id
                && reference.source_location_id.as_deref() == Some(location_id.as_str())
        }));
        assert_eq!(context_a.gaps.len(), 1);
        assert_eq!(context_a.gaps[0].id, gap_a.id);
        assert_eq!(
            context_a.gaps[0].literature_review_ids,
            vec![review.id.clone()]
        );
        assert_eq!(context_a.gaps[0].prior_assessment_ids, vec![assessment.id]);
        assert_eq!(context_a.questions.len(), 1);
        assert_eq!(context_a.questions[0].id, question.id);
        assert_eq!(
            context_a.questions[0].research_gap_id.as_deref(),
            Some(gap_a.id.as_str())
        );

        assert_eq!(context_b.project_id, project_b.id);
        assert!(context_b.prior_assessments.is_empty());
        assert_eq!(context_b.project_sources.len(), 0);
        assert!(context_b.literature_reviews.is_empty());
        assert!(context_b.source_versions.is_empty());
        assert_eq!(context_b.gaps.len(), 1);
        assert_eq!(context_b.gaps[0].id, gap_b.id);
        assert!(context_b.gaps[0].literature_review_ids.is_empty());
        assert!(context_b.gaps[0].prior_assessment_ids.is_empty());
        assert!(context_b.questions.is_empty());
        assert!(context_b
            .questions
            .iter()
            .all(|item| item.project_id.as_deref() == Some(project_b.id.as_str())));

        assert_eq!(unassigned.project_id, None);
        assert_eq!(unassigned.ownership_state, "unassigned");
        let global_workspace =
            invoke::<ResearchWorkspaceDto>(&window, "research_get_workspace", json!({}))
                .expect("retrieve global Research workspace through IPC");
        assert!(global_workspace.questions.iter().any(|item| {
            item.id == unassigned.id
                && item.project_id.is_none()
                && item.ownership_state == "unassigned"
        }));

        let missing_version = invoke::<LiteratureReviewDto>(
            &window,
            "research_create_reviewed_literature_review",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "title": "Missing exact version",
                    "searchScope": "Negative control.",
                    "sourceId": source_id,
                    "sourceVersionId": "unavailable-exact-version",
                    "sourceLocationId": null,
                    "sourceClaimId": null,
                    "citationNote": "",
                    "createdBy": null,
                    "reviewerId": null
                }
            }),
        )
        .expect_err("unavailable exact version must fail");
        assert!(error_text(missing_version).contains("Exact SourceVersion"));
        let wrong_source_version = invoke::<LiteratureReviewDto>(
            &window,
            "research_create_reviewed_literature_review",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "title": "Version from another Source",
                    "searchScope": "Negative control.",
                    "sourceId": source_id,
                    "sourceVersionId": other_version_id,
                    "sourceLocationId": null,
                    "sourceClaimId": null,
                    "citationNote": "",
                    "createdBy": null,
                    "reviewerId": null
                }
            }),
        )
        .expect_err("a SourceVersion from another Source must fail");
        assert!(error_text(wrong_source_version).contains("Exact SourceVersion"));
        assert_ne!(source_id, other_source_id);
        let missing_location = invoke::<LiteratureReviewDto>(
            &window,
            "research_create_reviewed_literature_review",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "title": "Missing exact location",
                    "searchScope": "Negative control.",
                    "sourceId": source_id,
                    "sourceVersionId": version_id,
                    "sourceLocationId": "unavailable-exact-location",
                    "sourceClaimId": null,
                    "citationNote": "",
                    "createdBy": null,
                    "reviewerId": null
                }
            }),
        )
        .expect_err("unavailable exact location must fail");
        assert!(error_text(missing_location).contains("SourceLocation"));
        let missing_claim = invoke::<LiteratureReviewDto>(
            &window,
            "research_create_reviewed_literature_review",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "title": "Missing exact claim",
                    "searchScope": "Negative control.",
                    "sourceId": source_id,
                    "sourceVersionId": version_id,
                    "sourceLocationId": location_id,
                    "sourceClaimId": "unavailable-exact-claim",
                    "citationNote": "",
                    "createdBy": null,
                    "reviewerId": null
                }
            }),
        )
        .expect_err("unavailable exact claim must fail");
        assert!(error_text(missing_claim).contains("SourceClaim"));
        let context_after_invalid_refs = invoke::<ResearchProjectContextDto>(
            &window,
            "research_get_project_context",
            json!({ "projectId": project_a.id }),
        )
        .expect("re-read Project A after invalid provenance requests");
        assert_eq!(context_after_invalid_refs.literature_reviews.len(), 2);

        let cross_project_evidence = invoke::<()>(
            &window,
            "research_add_gap_evidence",
            json!({
                "request": {
                    "gapId": gap_b.id,
                    "literatureReviewId": review.id,
                    "priorAssessmentId": null
                }
            }),
        )
        .expect_err("Project A review cannot be linked to Project B gap");
        assert!(error_text(cross_project_evidence).contains("belongs to project"));
        let unavailable_evidence = invoke::<()>(
            &window,
            "research_add_gap_evidence",
            json!({
                "request": {
                    "gapId": gap_a.id,
                    "literatureReviewId": "missing-review",
                    "priorAssessmentId": null
                }
            }),
        )
        .expect_err("unavailable review evidence must fail");
        assert!(error_text(unavailable_evidence)
            .contains("LiteratureReview `missing-review` was not found."));
        let cross_project_question = invoke::<ResearchQuestionDto>(
            &window,
            "research_create_question",
            json!({
                "request": {
                    "projectId": project_b.id,
                    "researchGapId": gap_a.id,
                    "domainId": null,
                    "title": "Invalid cross-project question",
                    "description": "Must not persist.",
                    "rationale": "Negative isolation control.",
                    "createdBy": null
                }
            }),
        )
        .expect_err("Project B question cannot link to Project A gap");
        assert!(error_text(cross_project_question).contains("belongs to project"));
        let missing_project = invoke::<ResearchProjectContextDto>(
            &window,
            "research_get_project_context",
            json!({ "projectId": "missing-project" }),
        )
        .expect_err("unknown project context must fail");
        assert!(error_text(missing_project).contains("was not found"));

        let archived_project = invoke::<ResearchProjectDto>(
            &window,
            "research_transition_project",
            json!({
                "projectId": project_b.id,
                "nextStatus": "archived",
                "actorId": "phase-8.2-test"
            }),
        )
        .expect("archive control project through IPC");
        assert_eq!(
            archived_project.lifecycle_status,
            ResearchProjectStatus::Archived
        );
        let archived_write = invoke::<PriorKnowledgeAssessmentDto>(
            &window,
            "research_create_prior_assessment",
            json!({
                "request": {
                    "projectId": project_b.id,
                    "title": "Must not persist in archived project",
                    "assessment": "Archived project write control.",
                    "frameworkVersionId": null,
                    "createdBy": "phase-8.2-test"
                }
            }),
        )
        .expect_err("archived projects must reject Research writes");
        assert!(!error_text(archived_write).is_empty());
        let archived_context = invoke::<ResearchProjectContextDto>(
            &window,
            "research_get_project_context",
            json!({ "projectId": project_b.id }),
        )
        .expect("read archived project context");
        assert!(archived_context.prior_assessments.is_empty());

        {
            let state = app.state::<AppState>();
            let mut conn = state.db.lock().expect("lock isolated test database");
            let placeholder = Connection::open_in_memory()
                .expect("open temporary connection while reopening isolated database");
            let previous = std::mem::replace(&mut *conn, placeholder);
            previous
                .close()
                .expect("close isolated database before reopening");
            let reopened =
                Connection::open(&database_file.path).expect("reopen isolated Research database");
            reopened
                .pragma_update(None, "foreign_keys", "ON")
                .expect("enable foreign keys after reopening");
            *conn = reopened;
        }
        let reopened_context_a = invoke::<ResearchProjectContextDto>(
            &window,
            "research_get_project_context",
            json!({ "projectId": project_a.id }),
        )
        .expect("reload Project A context from reopened database");
        assert_eq!(reopened_context_a.project.id, project_a.id);
        assert_eq!(reopened_context_a.project_sources.len(), 1);
        assert_eq!(reopened_context_a.project_sources[0].source_id, source_id);
        assert_eq!(reopened_context_a.literature_reviews.len(), 2);
        let reopened_review = reopened_context_a
            .literature_reviews
            .iter()
            .find(|item| item.id == review.id)
            .expect("exact reviewed citation survives database reopen");
        assert_eq!(reopened_review.sources[0].source_id, source_id);
        assert_eq!(reopened_review.sources[0].source_version_id, version_id);
        assert_eq!(
            reopened_review.sources[0].source_location_id.as_deref(),
            Some(location_id.as_str())
        );
        assert_eq!(
            reopened_review.sources[0].source_claim_id.as_deref(),
            Some(claim_id.as_str())
        );
        assert_eq!(reopened_context_a.prior_assessments.len(), 1);
        assert_eq!(
            reopened_context_a.prior_assessments[0].lifecycle_status,
            ReviewLifecycleStatus::Reviewed
        );
        assert_eq!(reopened_context_a.gaps.len(), 1);
        assert_eq!(
            reopened_context_a.gaps[0].lifecycle_status,
            ResearchGapStatus::Accepted
        );
        assert_eq!(reopened_context_a.questions.len(), 1);
        assert_eq!(
            reopened_context_a.questions[0].research_gap_id.as_deref(),
            Some(gap_a.id.as_str())
        );
        let reopened_context_b = invoke::<ResearchProjectContextDto>(
            &window,
            "research_get_project_context",
            json!({ "projectId": project_b.id }),
        )
        .expect("reload Project B context from reopened database");
        assert_eq!(
            reopened_context_b.project.lifecycle_status,
            ResearchProjectStatus::Archived
        );
        assert_eq!(reopened_context_b.gaps.len(), 1);
        assert!(reopened_context_b.literature_reviews.is_empty());
        assert!(reopened_context_b.prior_assessments.is_empty());

        let draft_assessment = invoke::<PriorKnowledgeAssessmentDto>(
            &window,
            "research_create_prior_assessment",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "title": "Review failure remains visible",
                    "assessment": "Synthetic draft left by a failed second operation.",
                    "frameworkVersionId": null,
                    "createdBy": "phase-8.2-test"
                }
            }),
        )
        .expect("create assessment before injected review failure");
        {
            let state = app.state::<AppState>();
            let conn = state.db.lock().expect("lock isolated test database");
            conn.execute_batch(
                "CREATE TRIGGER reject_phase_8_2_prior_review_event
                 BEFORE INSERT ON research_provenance_events
                 WHEN NEW.target_kind = 'prior_knowledge_assessment'
                   AND NEW.event_kind = 'reviewed'
                 BEGIN
                     SELECT RAISE(ABORT, 'synthetic assessment review failure');
                 END;
                 CREATE TRIGGER reject_phase_8_2_literature_review_event
                 BEFORE INSERT ON research_provenance_events
                 WHEN NEW.target_kind = 'literature_review'
                   AND NEW.event_kind = 'reviewed'
                 BEGIN
                     SELECT RAISE(ABORT, 'synthetic literature review failure');
                 END;",
            )
            .expect("install isolated persistence failure triggers");
        }
        let failed_assessment_review = invoke::<PriorKnowledgeAssessmentDto>(
            &window,
            "research_review_prior_assessment",
            json!({
                "assessmentId": draft_assessment.id,
                "reviewerId": "phase-8.2-reviewer"
            }),
        )
        .expect_err("failed review step must remain an explicit error");
        assert!(
            error_text(failed_assessment_review).contains("synthetic assessment review failure")
        );
        let context_after_failed_assessment_review = invoke::<ResearchProjectContextDto>(
            &window,
            "research_get_project_context",
            json!({ "projectId": project_a.id }),
        )
        .expect("reload project after the separate review step fails");
        assert_eq!(
            context_after_failed_assessment_review
                .prior_assessments
                .iter()
                .find(|item| item.id == draft_assessment.id)
                .expect("persisted draft remains visible")
                .lifecycle_status,
            ReviewLifecycleStatus::Draft
        );

        let failed_literature_review = invoke::<LiteratureReviewDto>(
            &window,
            "research_create_reviewed_literature_review",
            json!({
                "request": {
                    "projectId": project_a.id,
                    "title": "Atomic review rollback",
                    "searchScope": "Failure injected after citation insertion.",
                    "sourceId": source_id,
                    "sourceVersionId": version_id,
                    "sourceLocationId": location_id,
                    "sourceClaimId": claim_id,
                    "citationNote": "Must roll back with review.",
                    "createdBy": "phase-8.2-test",
                    "reviewerId": "phase-8.2-reviewer"
                }
            }),
        )
        .expect_err("review and citation must roll back together");
        assert!(
            error_text(failed_literature_review).contains("synthetic literature review failure")
        );
        let context_after_failed_literature_review = invoke::<ResearchProjectContextDto>(
            &window,
            "research_get_project_context",
            json!({ "projectId": project_a.id }),
        )
        .expect("reload project after atomic review creation fails");
        assert_eq!(
            context_after_failed_literature_review
                .literature_reviews
                .len(),
            2
        );
        let state = app.state::<AppState>();
        let conn = state.db.lock().expect("lock isolated test database");
        let rolled_back_reviews: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM literature_reviews WHERE title = ?1",
                ["Atomic review rollback"],
                |row| row.get(0),
            )
            .expect("count rolled-back literature reviews");
        let rolled_back_citations: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM literature_review_sources
                 WHERE literature_review_id IN (
                     SELECT id FROM literature_reviews WHERE title = ?1
                 )",
                ["Atomic review rollback"],
                |row| row.get(0),
            )
            .expect("count rolled-back citations");
        assert_eq!(rolled_back_reviews, 0);
        assert_eq!(rolled_back_citations, 0);
        drop(conn);
        let state = app.state::<AppState>();
        let mut conn = state.db.lock().expect("lock isolated test database");
        let placeholder = Connection::open_in_memory()
            .expect("open temporary connection before closing isolated database");
        let previous = std::mem::replace(&mut *conn, placeholder);
        previous
            .close()
            .expect("close isolated Research test database");
    }
}
