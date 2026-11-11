import React, { useEffect, useMemo, useState } from "react";
import { AlertCircle, RefreshCw } from "lucide-react";
import { learningService } from "@/mind/services/learningService";
import type {
  LearningAssessmentOutcome,
  LearningWorkspaceDto,
} from "@/types";

const fieldClass =
  "w-full rounded-lg border border-[#343741] bg-[#101114] px-3 py-2 text-sm text-white outline-none focus:border-[#e8ad55]";
const buttonClass =
  "rounded-lg bg-[#e8ad55] px-3 py-2 text-xs font-semibold text-[#17130d] disabled:opacity-40";
const quietButtonClass =
  "rounded-lg border border-[#343741] px-3 py-2 text-xs text-gray-200 disabled:opacity-40";

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}): React.JSX.Element {
  return (
    <label className="block space-y-1.5 text-xs font-medium text-gray-400">
      <span>{label}</span>
      {children}
    </label>
  );
}

function parseJson(raw: string, label: string): unknown {
  try {
    const parsed: unknown = JSON.parse(raw);
    return parsed;
  } catch {
    throw new Error(`${label} must be valid JSON.`);
  }
}

function assessmentOutcome(value: string): LearningAssessmentOutcome {
  switch (value) {
    case "pass":
    case "fail":
    case "inconclusive":
      return value;
    default:
      throw new Error("Choose a supported assessment outcome.");
  }
}

function formatTime(timestampUs: number): string {
  return new Date(timestampUs / 1000).toLocaleString();
}

export function LearningWorkspacePanel(): React.JSX.Element {
  const [workspace, setWorkspace] = useState<LearningWorkspaceDto | null>(null);
  const [projectId, setProjectId] = useState("");
  const [attemptId, setAttemptId] = useState("");
  const [submissionId, setSubmissionId] = useState("");
  const [ruleCompetencyId, setRuleCompetencyId] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = async (): Promise<void> => {
    setLoading(true);
    try {
      const result = await learningService.getWorkspace();
      setWorkspace(result);
      setError(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setLoading(false);
    }
  };

  const perform = async (action: () => Promise<unknown>): Promise<void> => {
    try {
      await action();
      await refresh();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  useEffect(() => {
    if (!workspace) return;
    if (!workspace.projects.some((project) => project.id === projectId)) {
      setProjectId(workspace.projects[0]?.id ?? "");
    }
    if (!workspace.attempts.some((attempt) => attempt.id === attemptId)) {
      setAttemptId("");
    }
    if (!workspace.reproductionSubmissions.some((submission) => submission.id === submissionId)) {
      setSubmissionId("");
    }
  }, [workspace, projectId, attemptId, submissionId]);

  const project = workspace?.projects.find((item) => item.id === projectId);
  const objectives = useMemo(
    () => workspace?.objectives.filter((item) => item.learningProjectId === projectId) ?? [],
    [workspace, projectId],
  );
  const assessments = useMemo(
    () => workspace?.assessments.filter((item) => item.learningProjectId === projectId) ?? [],
    [workspace, projectId],
  );
  const attempts = useMemo(
    () => workspace?.attempts.filter((item) => item.learningProjectId === projectId) ?? [],
    [workspace, projectId],
  );
  const competencies = useMemo(
    () => workspace?.competencies.filter((item) => item.learningProjectId === projectId) ?? [],
    [workspace, projectId],
  );
  const tasks = useMemo(
    () => workspace?.reproductionTasks.filter((item) => item.learningProjectId === projectId) ?? [],
    [workspace, projectId],
  );
  const selectedAttempt = workspace?.attempts.find((item) => item.id === attemptId);
  const selectedAttemptAssessment = assessments.find(
    (item) => item.id === selectedAttempt?.assessmentVersionId,
  );
  const selectedSubmission = workspace?.reproductionSubmissions.find(
    (item) => item.id === submissionId,
  );
  const selectedTask = tasks.find((item) => item.id === selectedSubmission?.taskId);
  const selectedReproductionRubric = assessments.find(
    (item) => item.rubricVersionId === selectedTask?.rubricVersionId,
  );

  const submitForm = (
    event: React.FormEvent<HTMLFormElement>,
    action: (form: FormData) => Promise<unknown>,
  ): void => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    void perform(() => action(form));
  };

  const createLearner = (form: FormData): Promise<unknown> =>
    learningService.createLearner({
      displayName: String(form.get("displayName") ?? ""),
      createdBy: null,
    });

  const createProject = (form: FormData): Promise<unknown> => {
    const frameworkVersionId = String(form.get("frameworkVersionId") ?? "");
    const frameworkVersion = workspace?.frameworkVersions.find(
      (item) => item.frameworkVersionId === frameworkVersionId,
    );
    if (!frameworkVersion) throw new Error("Choose a published framework version.");
    return learningService.createProject({
      title: String(form.get("title") ?? ""),
      learnerId: String(form.get("learnerId") ?? ""),
      frameworkId: frameworkVersion.frameworkId,
      frameworkVersionId,
      createdBy: null,
    });
  };

  const createObjective = (form: FormData): Promise<unknown> =>
    learningService.createObjective({
      objectiveId: null,
      learningProjectId: projectId,
      title: String(form.get("title") ?? ""),
      observableOutcome: String(form.get("observableOutcome") ?? ""),
      criteriaText: String(form.get("criteriaText") ?? ""),
      level: String(form.get("level") ?? ""),
      parentVersionId: null,
      prerequisiteVersionIds: form.getAll("prerequisiteVersionIds")
        .filter((value): value is string => typeof value === "string"),
      createdBy: null,
    });

  const createAssessment = (form: FormData): Promise<unknown> => {
    const objectiveVersionId = String(form.get("objectiveVersionId") ?? "");
    const questionMaxPoints = Number(form.get("questionMaxPoints"));
    const rubricMaxPoints = Number(form.get("rubricMaxPoints"));
    return learningService.createAssessmentVersion({
      assessmentId: String(form.get("assessmentId") ?? "") || null,
      learningProjectId: projectId,
      title: String(form.get("title") ?? ""),
      changeRationale: String(form.get("changeRationale") ?? ""),
      rubricTitle: String(form.get("rubricTitle") ?? ""),
      rubricPassThreshold: Number(form.get("passThreshold")) / 100,
      questions: [{
        objectiveVersionId,
        questionOrder: 0,
        prompt: String(form.get("prompt") ?? ""),
        responseKind: "text",
        maxPoints: questionMaxPoints,
        required: true,
      }],
      rubricCriteria: [{
        objectiveVersionId,
        criterionKey: String(form.get("criterionKey") ?? ""),
        description: String(form.get("criterionDescription") ?? ""),
        maxPoints: rubricMaxPoints,
      }],
      createdBy: null,
    });
  };

  const createAttempt = (form: FormData): Promise<unknown> =>
    learningService.createAttempt({
      learningProjectId: projectId,
      learnerId: project?.learnerId ?? "",
      assessmentVersionId: String(form.get("assessmentVersionId") ?? ""),
    });

  const evaluateAttempt = (form: FormData): Promise<unknown> => {
    if (!selectedAttempt || !selectedAttemptAssessment) {
      throw new Error("Choose an assessment attempt to evaluate.");
    }
    const evaluation = selectedAttempt.evaluations[selectedAttempt.evaluations.length - 1];
    return learningService.evaluateAttempt({
      attemptId,
      correctionOfId: evaluation?.id ?? null,
      evaluatorId: String(form.get("evaluatorId") ?? ""),
      outcome: assessmentOutcome(String(form.get("outcome") ?? "inconclusive")),
      criterionScores: selectedAttemptAssessment.rubricCriteria.map((criterion) => ({
        rubricCriterionId: criterion.id,
        score: Number(form.get(`score:${criterion.id}`)),
      })),
      rationale: String(form.get("rationale") ?? ""),
      feedback: String(form.get("feedback") ?? ""),
    });
  };

  const createCompetency = (form: FormData): Promise<unknown> =>
    learningService.createCompetency({
      learningProjectId: projectId,
      title: String(form.get("title") ?? ""),
      description: String(form.get("description") ?? ""),
      criteria: [{
        objectiveVersionId: String(form.get("objectiveVersionId") ?? ""),
        criterion: String(form.get("criterion") ?? ""),
        minimumScore: Number(form.get("minimumScore")) / 100,
        required: true,
      }],
      createdBy: null,
    });

  const createMasteryRule = (form: FormData): Promise<unknown> => {
    const masteryRuleId = String(form.get("masteryRuleId") ?? "") || null;
    const existingRule = workspace?.masteryRules.find((rule) => rule.id === masteryRuleId);
    return learningService.createMasteryRule({
      masteryRuleId,
      competencyId: String(form.get("competencyId") ?? ""),
      title: existingRule?.title ?? String(form.get("title") ?? ""),
      minimumAverageScore: Number(form.get("minimumAverageScore")) / 100,
      minimumEvaluatedAttempts: Number(form.get("minimumAttempts")),
      allRequiredCriteria: true,
      reproductionRequired: form.get("reproductionRequired") === "on",
      requiredReproductionOutcome: "pass",
      createdBy: null,
    });
  };

  const evaluateCompetency = (form: FormData): Promise<unknown> =>
    learningService.evaluateCompetency({
      competencyId: String(form.get("competencyId") ?? ""),
      learnerId: project?.learnerId ?? "",
      masteryRuleVersionId: String(form.get("masteryRuleVersionId") ?? ""),
    });

  const createReproductionTask = (form: FormData): Promise<unknown> => {
    const assessment = assessments.find(
      (item) => item.id === String(form.get("assessmentVersionId") ?? ""),
    );
    if (!assessment) throw new Error("Choose an assessment version for its pinned rubric.");
    return learningService.createReproductionTask({
      learningProjectId: projectId,
      objectiveVersionId: String(form.get("objectiveVersionId") ?? ""),
      rubricVersionId: assessment.rubricVersionId,
      title: String(form.get("title") ?? ""),
      taskDescription: String(form.get("taskDescription") ?? ""),
      conditions: String(form.get("conditions") ?? ""),
      expectedResult: parseJson(String(form.get("expectedResult") ?? ""), "Expected result"),
      createdBy: null,
    });
  };

  const submitReproduction = (form: FormData): Promise<unknown> =>
    learningService.submitReproduction({
      taskId: String(form.get("taskId") ?? ""),
      learnerId: project?.learnerId ?? "",
      artifactUri: null,
      artifactSha256: null,
      result: parseJson(String(form.get("result") ?? ""), "Submitted result"),
      deviations: String(form.get("deviations") ?? ""),
    });

  const evaluateReproduction = (form: FormData): Promise<unknown> => {
    if (!selectedSubmission || !selectedReproductionRubric) {
      throw new Error("Choose a reproduction submission to evaluate.");
    }
    return learningService.evaluateReproduction({
      submissionId,
      evaluatorId: String(form.get("evaluatorId") ?? ""),
      outcome: assessmentOutcome(String(form.get("outcome") ?? "inconclusive")),
      criterionScores: selectedReproductionRubric.rubricCriteria.map((criterion) => ({
        rubricCriterionId: criterion.id,
        score: Number(form.get(`score:${criterion.id}`)),
      })),
      rationale: String(form.get("rationale") ?? ""),
      evidenceDescription: String(form.get("evidenceDescription") ?? ""),
    });
  };

  if (loading && !workspace) {
    return <p className="rounded-xl border border-[#292c34] bg-[#17191f] p-5 text-sm text-gray-400">Loading Learning workspace…</p>;
  }

  return (
    <section className="space-y-5 rounded-xl border border-[#292c34] bg-[#17191f] p-4">
      <header className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-lg font-semibold text-white">Learning and explainable mastery</h2>
          <p className="mt-1 text-xs text-gray-400">
            Assessments pin a framework version; attempts and evaluations keep their history. Mastery is derived only from explicit rules and evaluated evidence.
          </p>
        </div>
        <button type="button" className={quietButtonClass} onClick={() => void refresh()} aria-label="Refresh Learning workspace">
          <RefreshCw size={14} />
        </button>
      </header>
      {error && (
        <div role="alert" className="flex items-start gap-2 rounded-lg border border-rose-900/60 bg-rose-950/30 p-3 text-xs text-rose-200">
          <AlertCircle size={15} /> <span>{error}</span>
        </div>
      )}
      {!workspace ? (
        <p className="text-sm text-gray-400">Learning records are unavailable. Refresh to try again.</p>
      ) : (
        <>
          <div className="grid gap-4 lg:grid-cols-2">
            <form onSubmit={(event) => submitForm(event, createLearner)} className="space-y-3 rounded-lg border border-[#292c34] bg-[#101114] p-3">
              <h3 className="text-sm font-semibold text-white">Learner</h3>
              <Field label="Display name"><input className={fieldClass} name="displayName" required /></Field>
              <button className={buttonClass}>Add learner</button>
              <p className="text-[11px] text-gray-500">{workspace.learners.length} learner(s) recorded</p>
            </form>
            <form onSubmit={(event) => submitForm(event, createProject)} className="space-y-3 rounded-lg border border-[#292c34] bg-[#101114] p-3">
              <h3 className="text-sm font-semibold text-white">Learning context</h3>
              <Field label="Project title"><input className={fieldClass} name="title" required /></Field>
              <Field label="Learner">
                <select className={fieldClass} name="learnerId" required>
                  <option value="">Select learner</option>
                  {workspace.learners.map((learner) => <option key={learner.id} value={learner.id}>{learner.displayName}</option>)}
                </select>
              </Field>
              <Field label="Exact published framework version">
                <select className={fieldClass} name="frameworkVersionId" required>
                  <option value="">Select version</option>
                  {workspace.frameworkVersions.map((version) => (
                    <option key={version.frameworkVersionId} value={version.frameworkVersionId}>{version.title} · v{version.versionNo}</option>
                  ))}
                </select>
              </Field>
              <button className={buttonClass} disabled={!workspace.learners.length || !workspace.frameworkVersions.length}>Create pinned project</button>
              {workspace.projects.length === 0 && !workspace.frameworkVersions.length && (
                <p className="text-xs text-amber-300">Publish a framework version before creating a Learning project.</p>
              )}
            </form>
          </div>

          <div className="grid gap-3 md:grid-cols-[minmax(0,1fr)_2fr]">
            <Field label="Learning project">
              <select className={fieldClass} value={projectId} onChange={(event) => setProjectId(event.target.value)}>
                <option value="">Select project</option>
                {workspace.projects.map((item) => (
                  <option key={item.id} value={item.id}>{item.title} · {item.learnerName}</option>
                ))}
              </select>
            </Field>
            {project && (
              <p className="self-end rounded-lg bg-[#101114] px-3 py-2 text-xs text-gray-400">
                Pinned framework: <span className="text-gray-200">{project.frameworkTitle} v{project.frameworkVersionNo}</span>
                <span className="ml-2 text-gray-600">({project.frameworkVersionId})</span>
              </p>
            )}
          </div>

          {project && (
            <>
              <div className="grid gap-4 lg:grid-cols-2">
                <form onSubmit={(event) => submitForm(event, createObjective)} className="space-y-3 rounded-lg border border-[#292c34] bg-[#101114] p-3">
                  <h3 className="text-sm font-semibold text-white">Learning objectives</h3>
                  <Field label="Objective title"><input className={fieldClass} name="title" required /></Field>
                  <Field label="Observable outcome"><textarea className={fieldClass} name="observableOutcome" rows={2} required /></Field>
                  <Field label="Assessment criteria"><textarea className={fieldClass} name="criteriaText" rows={2} required /></Field>
                  <Field label="Level"><input className={fieldClass} name="level" required placeholder="e.g. introductory" /></Field>
                  <Field label="Prerequisite objective versions (optional)">
                    <select className={fieldClass} name="prerequisiteVersionIds" multiple size={3}>
                      {objectives.map((objective) => (
                        <option key={objective.id} value={objective.id}>{objective.title} · v{objective.versionNo}</option>
                      ))}
                    </select>
                  </Field>
                  <button className={buttonClass}>Create objective version</button>
                  <div className="max-h-44 space-y-2 overflow-auto border-t border-[#292c34] pt-3">
                    {objectives.map((objective) => (
                      <article key={objective.id} className="rounded bg-[#17191f] p-2 text-xs">
                        <p className="font-medium text-gray-200">{objective.title} · v{objective.versionNo}</p>
                        <p className="mt-1 text-gray-400">{objective.observableOutcome}</p>
                        <p className="mt-1 text-[11px] text-gray-500">
                          Prerequisites: {objective.prerequisiteVersionIds.length
                            ? objective.prerequisiteVersionIds.map((id) => objectives.find((item) => item.id === id)?.title ?? id).join(", ")
                            : "none declared"}
                        </p>
                        <p className="mt-1 text-[10px] text-gray-600">Framework version {objective.frameworkVersionId}</p>
                      </article>
                    ))}
                    {!objectives.length && <p className="text-xs text-gray-500">No objectives recorded for this project.</p>}
                  </div>
                </form>

                <form onSubmit={(event) => submitForm(event, createAssessment)} className="space-y-3 rounded-lg border border-[#292c34] bg-[#101114] p-3">
                  <h3 className="text-sm font-semibold text-white">Assessment authoring</h3>
                  <Field label="Assessment identity">
                    <select className={fieldClass} name="assessmentId">
                      <option value="">Create new assessment</option>
                      {assessments.filter((item, index, versions) =>
                        versions.findIndex((candidate) => candidate.assessmentId === item.assessmentId) === index
                      ).map((item) => (
                        <option key={item.assessmentId} value={item.assessmentId}>New version of {item.title}</option>
                      ))}
                    </select>
                  </Field>
                  <Field label="Objective version">
                    <select className={fieldClass} name="objectiveVersionId" required>
                      <option value="">Select objective</option>
                      {objectives.map((objective) => <option key={objective.id} value={objective.id}>{objective.title} · v{objective.versionNo}</option>)}
                    </select>
                  </Field>
                  <Field label="Assessment title"><input className={fieldClass} name="title" required /></Field>
                  <Field label="Question"><textarea className={fieldClass} name="prompt" rows={2} required /></Field>
                  <div className="grid grid-cols-2 gap-2">
                    <Field label="Question maximum points"><input className={fieldClass} type="number" min="0.01" step="any" name="questionMaxPoints" defaultValue="1" required /></Field>
                    <Field label="Pass threshold (%)"><input className={fieldClass} type="number" min="0" max="100" step="any" name="passThreshold" defaultValue="70" required /></Field>
                  </div>
                  <Field label="Rubric title"><input className={fieldClass} name="rubricTitle" required /></Field>
                  <div className="grid grid-cols-2 gap-2">
                    <Field label="Criterion key"><input className={fieldClass} name="criterionKey" required /></Field>
                    <Field label="Criterion maximum points"><input className={fieldClass} type="number" min="0.01" step="any" name="rubricMaxPoints" defaultValue="1" required /></Field>
                  </div>
                  <Field label="Rubric description"><input className={fieldClass} name="criterionDescription" required /></Field>
                  <Field label="Version rationale"><input className={fieldClass} name="changeRationale" required /></Field>
                  <button className={buttonClass} disabled={!objectives.length}>Save draft assessment</button>
                </form>
              </div>

              <section className="space-y-3 rounded-lg border border-[#292c34] bg-[#101114] p-3">
                <h3 className="text-sm font-semibold text-white">Assessment versions and review</h3>
                <div className="grid gap-2 md:grid-cols-2">
                  {assessments.map((assessment) => (
                    <article key={assessment.id} className="rounded-lg border border-[#292c34] bg-[#17191f] p-3">
                      <div className="flex flex-wrap items-center justify-between gap-2">
                        <p className="text-sm text-gray-200">{assessment.title} · v{assessment.versionNo}</p>
                        <span className="rounded bg-[#242630] px-2 py-1 text-[10px] text-gray-300">{assessment.status}</span>
                      </div>
                      <p className="mt-1 text-[11px] text-gray-500">Framework {assessment.frameworkVersionId} · rubric {assessment.rubricVersionId}</p>
                      {assessment.reviews.map((review) => <p key={review.id} className="mt-1 text-[11px] text-gray-400">Review: {review.reviewStatus} · {review.rationale}</p>)}
                      {assessment.status === "draft" && (
                        <button type="button" className={`${quietButtonClass} mt-2`} onClick={() => void perform(() => learningService.reviewAssessment({
                          assessmentVersionId: assessment.id,
                          reviewStatus: "approved",
                          reviewerId: null,
                          rationale: "Approved in the Learning workspace.",
                        }))}>Approve review</button>
                      )}
                      {assessment.status === "review"
                        && assessment.reviews[assessment.reviews.length - 1]?.reviewStatus === "approved" && (
                        <button type="button" className={`${quietButtonClass} mt-2`} onClick={() => void perform(() => learningService.transitionAssessmentVersion({
                          assessmentVersionId: assessment.id,
                          nextStatus: "published",
                          actorId: null,
                        }))}>Publish immutable version</button>
                      )}
                    </article>
                  ))}
                  {!assessments.length && <p className="text-xs text-gray-500">No assessments recorded for this project.</p>}
                </div>
              </section>

              <div className="grid gap-4 lg:grid-cols-2">
                <section className="space-y-3 rounded-lg border border-[#292c34] bg-[#101114] p-3">
                  <h3 className="text-sm font-semibold text-white">Assessment delivery and evaluation</h3>
                  <form onSubmit={(event) => submitForm(event, createAttempt)} className="flex flex-wrap items-end gap-2">
                    <Field label="Published assessment version">
                      <select className={fieldClass} name="assessmentVersionId" required>
                        <option value="">Select published assessment</option>
                        {assessments.filter((item) => item.status === "published").map((item) => <option key={item.id} value={item.id}>{item.title} · v{item.versionNo}</option>)}
                      </select>
                    </Field>
                    <button className={buttonClass} disabled={!assessments.some((item) => item.status === "published")}>Start attempt</button>
                  </form>
                  <Field label="Attempt history">
                    <select className={fieldClass} value={attemptId} onChange={(event) => setAttemptId(event.target.value)}>
                      <option value="">Select attempt</option>
                      {attempts.map((attempt) => <option key={attempt.id} value={attempt.id}>{attempt.id} · {attempt.status} · framework {attempt.frameworkVersionId}</option>)}
                    </select>
                  </Field>
                  {selectedAttempt && selectedAttemptAssessment && (
                    <div className="space-y-3 rounded bg-[#17191f] p-3">
                      <p className="text-[11px] text-gray-500">Pinned assessment {selectedAttempt.assessmentVersionId} · framework {selectedAttempt.frameworkVersionId} · started {formatTime(selectedAttempt.startedAtUs)}</p>
                      {selectedAttempt.status === "in_progress" && selectedAttemptAssessment.questions.map((question) => {
                        const saved = selectedAttempt.responses.find((response) => response.assessmentQuestionId === question.id);
                        return (
                          <form key={question.id} onSubmit={(event) => {
                            event.preventDefault();
                            const values = new FormData(event.currentTarget);
                            void perform(() => learningService.saveAttemptResponse({
                              attemptId,
                              assessmentQuestionId: question.id,
                              response: String(values.get("response") ?? ""),
                            }));
                          }} className="space-y-2">
                            <Field label={question.prompt}>
                              <textarea className={fieldClass} name="response" rows={2} defaultValue={typeof saved?.response === "string" ? saved.response : ""} required={question.required} />
                            </Field>
                            <button className={quietButtonClass}>Save response</button>
                          </form>
                        );
                      })}
                      {selectedAttempt.status === "in_progress" && (
                        <button type="button" className={buttonClass} onClick={() => void perform(() => learningService.submitAttempt(attemptId))}>Submit immutable attempt</button>
                      )}
                      {selectedAttempt.status === "submitted" && (
                        <form onSubmit={(event) => submitForm(event, evaluateAttempt)} className="space-y-2 border-t border-[#292c34] pt-3">
                          <p className="text-xs font-medium text-gray-300">{selectedAttempt.evaluations.length ? "Correction creates a new evaluation revision." : "Evaluation is recorded separately from submission."}</p>
                          <Field label="Evaluator identity"><input className={fieldClass} name="evaluatorId" required /></Field>
                          {selectedAttemptAssessment.rubricCriteria.map((criterion) => (
                            <Field key={criterion.id} label={`${criterion.description} (0–${criterion.maxPoints})`}>
                              <input className={fieldClass} type="number" min="0" max={criterion.maxPoints} step="any" name={`score:${criterion.id}`} required />
                            </Field>
                          ))}
                          <Field label="Outcome">
                            <select className={fieldClass} name="outcome"><option value="pass">Pass</option><option value="fail">Fail</option><option value="inconclusive">Inconclusive</option></select>
                          </Field>
                          <Field label="Rationale"><input className={fieldClass} name="rationale" required /></Field>
                          <Field label="Feedback"><textarea className={fieldClass} name="feedback" rows={2} required /></Field>
                          <button className={buttonClass}>Record evaluation revision</button>
                        </form>
                      )}
                      {selectedAttempt.evaluations.map((evaluation) => (
                        <p key={evaluation.id} className="text-[11px] text-gray-400">
                          Evaluation r{evaluation.revisionNo}: {evaluation.outcome}, {Math.round(evaluation.score * 100)}% · {evaluation.evaluatorId} · {evaluation.correctionOfId ? `corrects ${evaluation.correctionOfId}` : "original"}
                        </p>
                      ))}
                    </div>
                  )}
                  {!attempts.length && <p className="text-xs text-gray-500">No attempts recorded.</p>}
                </section>

                <section className="space-y-3 rounded-lg border border-[#292c34] bg-[#101114] p-3">
                  <h3 className="text-sm font-semibold text-white">Competency and mastery</h3>
                  <form onSubmit={(event) => submitForm(event, createCompetency)} className="space-y-2 border-b border-[#292c34] pb-3">
                    <Field label="Competency title"><input className={fieldClass} name="title" required /></Field>
                    <Field label="Description"><input className={fieldClass} name="description" /></Field>
                    <Field label="Objective version"><select className={fieldClass} name="objectiveVersionId" required><option value="">Select objective</option>{objectives.map((objective) => <option key={objective.id} value={objective.id}>{objective.title} · v{objective.versionNo}</option>)}</select></Field>
                    <Field label="Required criterion"><input className={fieldClass} name="criterion" required /></Field>
                    <Field label="Minimum criterion score (%)"><input className={fieldClass} type="number" min="0" max="100" name="minimumScore" defaultValue="70" required /></Field>
                    <button className={buttonClass} disabled={!objectives.length}>Create competency</button>
                  </form>
                  <form onSubmit={(event) => submitForm(event, createMasteryRule)} className="space-y-2 border-b border-[#292c34] pb-3">
                    <Field label="Competency">
                      <select className={fieldClass} name="competencyId" value={ruleCompetencyId} onChange={(event) => setRuleCompetencyId(event.target.value)} required>
                        <option value="">Select competency</option>
                        {competencies.map((item) => <option key={item.id} value={item.id}>{item.title}</option>)}
                      </select>
                    </Field>
                    <Field label="Rule identity">
                      <select className={fieldClass} name="masteryRuleId">
                        <option value="">Create new mastery rule</option>
                        {workspace.masteryRules.filter((rule) => rule.competencyId === ruleCompetencyId).map((rule) => (
                          <option key={rule.id} value={rule.id}>New version of {rule.title}</option>
                        ))}
                      </select>
                    </Field>
                    <Field label="Rule title (for a new identity)"><input className={fieldClass} name="title" required /></Field>
                    <div className="grid grid-cols-2 gap-2">
                      <Field label="Minimum average (%)"><input className={fieldClass} type="number" min="0" max="100" name="minimumAverageScore" defaultValue="70" required /></Field>
                      <Field label="Evaluated attempts"><input className={fieldClass} type="number" min="1" step="1" name="minimumAttempts" defaultValue="1" required /></Field>
                    </div>
                    <label className="flex items-center gap-2 text-xs text-gray-300"><input type="checkbox" name="reproductionRequired" /> Require independently evaluated reproduction</label>
                    <button className={buttonClass} disabled={!competencies.length}>Save mastery rule version</button>
                  </form>
                  <form onSubmit={(event) => submitForm(event, evaluateCompetency)} className="space-y-2">
                    <Field label="Competency"><select className={fieldClass} name="competencyId" required><option value="">Select competency</option>{competencies.map((item) => <option key={item.id} value={item.id}>{item.title}</option>)}</select></Field>
                    <Field label="Pinned mastery-rule version"><select className={fieldClass} name="masteryRuleVersionId" required><option value="">Select rule version</option>{workspace.masteryRules.filter((rule) => competencies.some((item) => item.id === rule.competencyId)).flatMap((rule) => rule.versions.map((version) => <option key={version.id} value={version.id}>{rule.title} · v{version.versionNo}</option>))}</select></Field>
                    <button className={buttonClass} disabled={!workspace.masteryRules.length}>Evaluate recorded evidence</button>
                  </form>
                  {workspace.competencyRecords.filter((record) => record.learningProjectId === projectId).map((record) => (
                    <article key={record.id} className="rounded bg-[#17191f] p-2 text-xs">
                      <p className={record.outcome === "mastered" ? "text-emerald-300" : "text-amber-300"}>{record.outcome.replace("_", " ")} · rule {record.masteryRuleVersionId}</p>
                      <p className="mt-1 text-gray-400">{record.explanation}</p>
                      <p className="mt-1 text-[10px] text-gray-500">
                        Objective versions: {record.objectiveVersionIds.join(", ") || "none"}
                        {" · "}Assessment evaluations: {record.attemptEvaluationIds.join(", ") || "none"}
                        {" · "}Reproduction evaluations: {record.reproductionEvaluationIds.join(", ") || "none"}
                      </p>
                      {record.criteria.map((criterion) => (
                        <p key={criterion.competencyCriterionId} className="mt-1 text-[10px] text-gray-500">
                          {criterion.satisfied ? "Satisfied" : "Not satisfied"}: {criterion.rationale}
                        </p>
                      ))}
                    </article>
                  ))}
                </section>
              </div>

              <div className="grid gap-4 lg:grid-cols-2">
                <section className="space-y-3 rounded-lg border border-[#292c34] bg-[#101114] p-3">
                  <h3 className="text-sm font-semibold text-white">Independent reproduction</h3>
                  <form onSubmit={(event) => submitForm(event, createReproductionTask)} className="space-y-2 border-b border-[#292c34] pb-3">
                    <Field label="Objective version"><select className={fieldClass} name="objectiveVersionId" required><option value="">Select objective</option>{objectives.map((objective) => <option key={objective.id} value={objective.id}>{objective.title}</option>)}</select></Field>
                    <Field label="Rubric source assessment"><select className={fieldClass} name="assessmentVersionId" required><option value="">Select assessment</option>{assessments.map((item) => <option key={item.id} value={item.id}>{item.title} · {item.status}</option>)}</select></Field>
                    <Field label="Task title"><input className={fieldClass} name="title" required /></Field>
                    <Field label="Task"><textarea className={fieldClass} name="taskDescription" rows={2} required /></Field>
                    <Field label="Conditions"><input className={fieldClass} name="conditions" required /></Field>
                    <Field label="Expected result (JSON)"><textarea className={fieldClass} name="expectedResult" rows={2} required placeholder={'{"expected": "..."}'} /></Field>
                    <button className={buttonClass} disabled={!objectives.length || !assessments.length}>Create reproduction task</button>
                  </form>
                  <form onSubmit={(event) => submitForm(event, submitReproduction)} className="space-y-2">
                    <Field label="Task"><select className={fieldClass} name="taskId" required><option value="">Select reproduction task</option>{tasks.map((task) => <option key={task.id} value={task.id}>{task.title}</option>)}</select></Field>
                    <Field label="Submitted result (JSON)"><textarea className={fieldClass} name="result" rows={2} required placeholder={'{"result": "..."}'} /></Field>
                    <Field label="Deviations"><input className={fieldClass} name="deviations" /></Field>
                    <button className={buttonClass} disabled={!tasks.length}>Submit result for independent evaluation</button>
                  </form>
                  <div className="space-y-1 border-t border-[#292c34] pt-2">
                    {workspace.reproductionSubmissions.filter((item) => tasks.some((task) => task.id === item.taskId)).map((item) => (
                      <p key={item.id} className="text-[11px] text-gray-400">
                        Submission {item.id} · {item.evaluations.length ? item.evaluations.map((evaluation) => evaluation.outcome).join(", ") : "awaiting evaluation"} · {formatTime(item.submittedAtUs)}
                      </p>
                    ))}
                  </div>
                </section>

                <section className="space-y-3 rounded-lg border border-[#292c34] bg-[#101114] p-3">
                  <h3 className="text-sm font-semibold text-white">Reproduction evaluation</h3>
                  <Field label="Submission">
                    <select className={fieldClass} value={submissionId} onChange={(event) => setSubmissionId(event.target.value)}>
                      <option value="">Select submission</option>
                      {workspace.reproductionSubmissions.filter((item) => tasks.some((task) => task.id === item.taskId)).map((item) => (
                        <option key={item.id} value={item.id}>{item.id} · {item.evaluations.length ? "evaluated" : "awaiting evaluation"}</option>
                      ))}
                    </select>
                  </Field>
                  {selectedSubmission && selectedReproductionRubric && (
                    <form onSubmit={(event) => submitForm(event, evaluateReproduction)} className="space-y-2">
                      <p className="text-[11px] text-gray-500">Submission by {selectedSubmission.learnerId}; evaluator must be a different identity. Rubric {selectedTask?.rubricVersionId}.</p>
                      <Field label="Evaluator identity"><input className={fieldClass} name="evaluatorId" required /></Field>
                      {selectedReproductionRubric.rubricCriteria.map((criterion) => (
                        <Field key={criterion.id} label={`${criterion.description} (0–${criterion.maxPoints})`}>
                          <input className={fieldClass} type="number" min="0" max={criterion.maxPoints} step="any" name={`score:${criterion.id}`} required />
                        </Field>
                      ))}
                      <Field label="Outcome"><select className={fieldClass} name="outcome"><option value="pass">Pass</option><option value="fail">Fail</option><option value="inconclusive">Inconclusive</option></select></Field>
                      <Field label="Rationale"><input className={fieldClass} name="rationale" required /></Field>
                      <Field label="Evidence / artifact inspected"><input className={fieldClass} name="evidenceDescription" required /></Field>
                      <button className={buttonClass}>Record independent evaluation</button>
                    </form>
                  )}
                  {!selectedSubmission && <p className="text-xs text-gray-500">Select a learner submission to inspect its evidence and record an independent evaluation.</p>}
                  {selectedSubmission?.evaluations.map((evaluation) => (
                    <p key={evaluation.id} className="text-[11px] text-gray-400">{evaluation.outcome} · {evaluation.evaluatorId} · {evaluation.evidenceDescription}</p>
                  ))}
                </section>
              </div>

              <section className="space-y-2 rounded-lg border border-[#292c34] bg-[#101114] p-3">
                <h3 className="text-sm font-semibold text-white">Explainable progress and history</h3>
                {workspace.progress.filter((item) => item.learningProjectId === projectId).map((item) => {
                  const objective = objectives.find((candidate) => candidate.id === item.objectiveVersionId);
                  return (
                    <article key={item.objectiveVersionId} className="grid gap-2 rounded bg-[#17191f] p-2 md:grid-cols-[1fr_auto]">
                      <div>
                        <p className="text-xs font-medium text-gray-200">{objective?.title ?? item.objectiveVersionId}</p>
                        <p className="mt-1 text-[11px] text-gray-400">{item.explanation}</p>
                        <p className="mt-1 text-[10px] text-gray-600">Framework {objective?.frameworkVersionId ?? project.frameworkVersionId}</p>
                      </div>
                      <span className="h-fit rounded bg-[#242630] px-2 py-1 text-[10px] text-gray-300">{item.status}</span>
                    </article>
                  );
                })}
                {!workspace.progress.some((item) => item.learningProjectId === projectId) && (
                  <p className="text-xs text-gray-500">No progress is inferred from activity or completion. Evaluated records will appear here with their explanation.</p>
                )}
                {attempts.map((attempt) => (
                  <p key={attempt.id} className="text-[10px] text-gray-600">
                    Historical attempt {attempt.id} · assessment {attempt.assessmentVersionId} · submitted {attempt.submittedAtUs ? formatTime(attempt.submittedAtUs) : "not submitted"}
                  </p>
                ))}
              </section>
            </>
          )}
        </>
      )}
    </section>
  );
}
