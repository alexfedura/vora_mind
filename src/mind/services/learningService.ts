import { invoke } from "@tauri-apps/api/core";
import type {
  AssessmentAttemptDto,
  AssessmentVersionDto,
  AttemptEvaluationDto,
  AttemptResponseDto,
  CompetencyDto,
  CompetencyRecordDto,
  CreateAssessmentAttemptReq,
  CreateAssessmentVersionReq,
  CreateLearnerReq,
  CreateLearningObjectiveReq,
  CreateLearningProjectReq,
  CreateMasteryRuleVersionReq,
  CreateReproductionTaskReq,
  CreateCompetencyReq,
  EvaluateAssessmentAttemptReq,
  EvaluateCompetencyReq,
  EvaluateReproductionReq,
  LearningLearnerDto,
  LearningProjectDto,
  LearningWorkspaceDto,
  MasteryRuleDto,
  ReproductionEvaluationDto,
  ReproductionSubmissionDto,
  ReproductionTaskDto,
  ReviewAssessmentReq,
  SaveAttemptResponseReq,
  SubmitReproductionReq,
  TransitionAssessmentVersionReq,
} from "@/types";

export const learningService = {
  getWorkspace: () => invoke<LearningWorkspaceDto>("learning_get_workspace"),
  createLearner: (request: CreateLearnerReq) =>
    invoke<LearningLearnerDto>("learning_create_learner", { request }),
  createProject: (request: CreateLearningProjectReq) =>
    invoke<LearningProjectDto>("learning_create_project", { request }),
  createObjective: (request: CreateLearningObjectiveReq) =>
    invoke("learning_create_objective", { request }),
  createAssessmentVersion: (request: CreateAssessmentVersionReq) =>
    invoke<AssessmentVersionDto>("learning_create_assessment_version", { request }),
  reviewAssessment: (request: ReviewAssessmentReq) =>
    invoke<AssessmentVersionDto>("learning_review_assessment", { request }),
  transitionAssessmentVersion: (request: TransitionAssessmentVersionReq) =>
    invoke<AssessmentVersionDto>("learning_transition_assessment_version", { request }),
  createAttempt: (request: CreateAssessmentAttemptReq) =>
    invoke<AssessmentAttemptDto>("learning_create_attempt", { request }),
  saveAttemptResponse: (request: SaveAttemptResponseReq) =>
    invoke<AttemptResponseDto>("learning_save_attempt_response", { request }),
  submitAttempt: (attemptId: string) =>
    invoke<AssessmentAttemptDto>("learning_submit_attempt", { attemptId }),
  evaluateAttempt: (request: EvaluateAssessmentAttemptReq) =>
    invoke<AttemptEvaluationDto>("learning_evaluate_attempt", { request }),
  createCompetency: (request: CreateCompetencyReq) =>
    invoke<CompetencyDto>("learning_create_competency", { request }),
  createMasteryRule: (request: CreateMasteryRuleVersionReq) =>
    invoke<MasteryRuleDto>("learning_create_mastery_rule", { request }),
  evaluateCompetency: (request: EvaluateCompetencyReq) =>
    invoke<CompetencyRecordDto>("learning_evaluate_competency", { request }),
  createReproductionTask: (request: CreateReproductionTaskReq) =>
    invoke<ReproductionTaskDto>("learning_create_reproduction_task", { request }),
  submitReproduction: (request: SubmitReproductionReq) =>
    invoke<ReproductionSubmissionDto>("learning_submit_reproduction", { request }),
  evaluateReproduction: (request: EvaluateReproductionReq) =>
    invoke<ReproductionEvaluationDto>("learning_evaluate_reproduction", { request }),
};
