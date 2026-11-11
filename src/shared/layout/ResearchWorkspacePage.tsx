import React, { useEffect, useMemo, useState } from "react";
import { AlertCircle, BookOpenCheck, Plus, RefreshCw } from "lucide-react";
import { Link } from "react-router-dom";
import { useResearchWorkspace } from "@/hooks/useResearchWorkspace";
import { researchService } from "@/mind/services/researchService";
import type {
  ResearchGapDto,
  HypothesisExpectedDirection,
  ObservationValueState,
  ResearchPathwayKey,
  ResearchProjectDto,
  ResearchQuestionDto,
  SourceVersionReferenceDto,
} from "@/types";

const PATHWAYS: Array<{ key: ResearchPathwayKey; label: string }> = [
  { key: "existing_knowledge_acquisition", label: "Acquire existing knowledge" },
  { key: "existing_knowledge_investigation", label: "Investigate existing knowledge" },
  { key: "first_principles_discovery", label: "First-principles discovery" },
];

const inputClass =
  "w-full rounded-lg border border-[#343741] bg-[#101114] px-3 py-2 text-sm text-white outline-none focus:border-[#e8ad55]";
const buttonClass =
  "inline-flex items-center gap-2 rounded-lg bg-[#e8ad55] px-3 py-2 text-sm font-semibold text-[#17130d] disabled:cursor-not-allowed disabled:opacity-40";

function isoDate(timestampUs: number): string {
  return new Date(timestampUs / 1000).toLocaleString();
}

function latest<T>(items: T[]): T | undefined {
  return items[items.length - 1];
}

function observationValueState(value: string): ObservationValueState {
  switch (value) {
    case "observed":
    case "not_measured":
    case "not_recorded":
    case "unknown":
    case "not_applicable":
    case "withheld":
    case "invalid":
      return value;
    default:
      throw new Error(`Unsupported observation value state: ${value}`);
  }
}

function hypothesisDirection(value: string): HypothesisExpectedDirection {
  switch (value) {
    case "positive":
    case "negative":
    case "non_directional":
    case "equivalence":
    case "other":
      return value;
    default:
      throw new Error(`Unsupported expected direction: ${value}`);
  }
}

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

function ProjectSelect({
  projects,
  value,
  onChange,
}: {
  projects: ResearchProjectDto[];
  value: string;
  onChange: (value: string) => void;
}): React.JSX.Element {
  return (
    <Field label="Research project">
      <select className={inputClass} value={value} onChange={(event) => onChange(event.target.value)} required>
        <option value="">Select a project</option>
        {projects.map((project) => (
          <option key={project.id} value={project.id}>{project.title}</option>
        ))}
      </select>
    </Field>
  );
}

export const ResearchWorkspacePage: React.FC = () => {
  const { workspace, isLoading, error, refresh, run } = useResearchWorkspace();
  const [projectId, setProjectId] = useState("");
  const [pathways, setPathways] = useState<ResearchPathwayKey[]>(["existing_knowledge_acquisition"]);
  const [questionProjectId, setQuestionProjectId] = useState("");
  const [selectedSourceKey, setSelectedSourceKey] = useState("");
  const [selectedPlanVersionId, setSelectedPlanVersionId] = useState("");
  const [selectedDatasetVersionId, setSelectedDatasetVersionId] = useState("");
  const [selectedHypothesisId, setSelectedHypothesisId] = useState("");

  const sourceVersions = workspace?.sourceVersions ?? [];
  const selectedSource = useMemo(
    () => sourceVersions.find((source) => sourceKey(source) === selectedSourceKey),
    [sourceVersions, selectedSourceKey],
  );
  const projectQuestions = (workspace?.questions ?? []).filter(
    (question) => question.projectId === questionProjectId,
  );
  const projectPhenomena = (workspace?.phenomena ?? []).filter(
    (phenomenon) => phenomenon.projectId === projectId,
  );
  const projectDomains = (workspace?.domains ?? []).filter((domain) =>
    workspace?.projects.find((project) => project.id === projectId)?.domainIds.includes(domain.id),
  );
  const questionProjectDomains = (workspace?.domains ?? []).filter((domain) =>
    workspace?.projects.find((project) => project.id === questionProjectId)?.domainIds.includes(domain.id),
  );
  const projectObservations = (workspace?.observations ?? []).filter(
    (observation) => observation.projectId === projectId,
  );
  const projectHypotheses = (workspace?.hypotheses ?? []).filter(
    (hypothesis) => hypothesis.projectId === projectId,
  );

  useEffect(() => {
    if (!workspace) return;
    if (!workspace.projects.some((project) => project.id === projectId)) {
      setProjectId(workspace.projects[0]?.id ?? "");
    }
    if (!workspace.projects.some((project) => project.id === questionProjectId)) {
      setQuestionProjectId(workspace.projects[0]?.id ?? "");
    }
  }, [workspace, projectId, questionProjectId]);

  const act = async (action: () => Promise<unknown>): Promise<void> => {
    try {
      await run(action);
    } catch {
      // The hook exposes the failure in the page-level alert.
    }
  };

  const createProject = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    try {
      const project = await run(() => researchService.createProject({
        title: String(form.get("title") ?? ""),
        aim: String(form.get("aim") ?? ""),
        pathways,
        createdBy: null,
      }));
      setProjectId(project.id);
      setQuestionProjectId(project.id);
      target.reset();
    } catch {
      // The hook exposes the failure in the page-level alert.
    }
  };

  const createDomain = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!projectId) return;
    const target = event.currentTarget;
    const form = new FormData(target);
    await act(async () => {
      const domain = await researchService.createDomain({
        name: String(form.get("name") ?? ""),
        description: String(form.get("description") ?? ""),
        parentDomainId: null,
        createdBy: null,
      });
      await researchService.assignDomain(projectId, domain.id, "Assigned to this research project.");
    });
    target.reset();
  };

  const adoptFrameworkVersion = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    const version = workspace?.frameworkVersions.find(
      (item) => item.frameworkVersionId === String(form.get("frameworkVersionId") ?? ""),
    );
    if (!version) return;
    await act(() => researchService.adoptFrameworkVersion(
      projectId,
      version.frameworkId,
      version.frameworkVersionId,
      String(form.get("context") ?? ""),
      null,
    ));
    target.reset();
  };

  const createPriorAssessment = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    await act(async () => {
      const assessment = await researchService.createPriorAssessment({
        projectId,
        title: String(form.get("title") ?? ""),
        assessment: String(form.get("assessment") ?? ""),
        frameworkVersionId: null,
        createdBy: null,
      });
      await researchService.reviewPriorAssessment(assessment.id, null);
    });
    target.reset();
  };

  const createLiteratureReview = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!selectedSource) return;
    const target = event.currentTarget;
    const form = new FormData(target);
    await act(() => researchService.createReviewedLiteratureReview({
        projectId,
        title: String(form.get("title") ?? ""),
        searchScope: String(form.get("scope") ?? ""),
        sourceId: selectedSource.sourceId,
        sourceVersionId: selectedSource.sourceVersionId,
        sourceLocationId: selectedSource.sourceLocationId,
        sourceClaimId: selectedSource.sourceClaimId,
        citationNote: String(form.get("citationNote") ?? ""),
        createdBy: null,
        reviewerId: null,
      }));
    target.reset();
  };

  const createGap = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    try {
      const domainId = String(form.get("domainId") ?? "");
      const gap = await run(() => researchService.createGap({
        projectId,
        domainId: domainId || null,
        title: String(form.get("title") ?? ""),
        statement: String(form.get("statement") ?? ""),
        createdBy: null,
      }));
      const evidenceId = String(form.get("evidenceId") ?? "");
      const evidenceKind = String(form.get("evidenceKind") ?? "");
      if (evidenceId) {
        await run(() => researchService.addGapEvidence({
        gapId: gap.id,
        literatureReviewId: evidenceKind === "literature" ? evidenceId : null,
        priorAssessmentId: evidenceKind === "prior" ? evidenceId : null,
        }));
      }
      target.reset();
    } catch {
      // The hook exposes the failure in the page-level alert.
    }
  };

  const addGapEvidence = async (gap: ResearchGapDto, form: HTMLFormElement): Promise<void> => {
    const values = new FormData(form);
    const evidenceId = String(values.get("evidenceId") ?? "");
    const evidenceKind = String(values.get("evidenceKind") ?? "");
    if (!evidenceId) return;
    await act(() => researchService.addGapEvidence({
      gapId: gap.id,
      literatureReviewId: evidenceKind === "literature" ? evidenceId : null,
      priorAssessmentId: evidenceKind === "prior" ? evidenceId : null,
    }));
  };

  const createQuestion = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    const gapId = String(form.get("gapId") ?? "");
    await act(() => researchService.createQuestion({
      projectId: questionProjectId || null,
      researchGapId: gapId || null,
      domainId: String(form.get("domainId") ?? "") || null,
      title: String(form.get("title") ?? ""),
      description: String(form.get("description") ?? ""),
      rationale: String(form.get("rationale") ?? ""),
      createdBy: null,
    }));
    target.reset();
  };

  const createPhenomenon = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    await act(() => researchService.createPhenomenon({
      projectId,
      domainId: String(form.get("domainId") ?? "") || null,
      title: String(form.get("title") ?? ""),
      description: String(form.get("description") ?? ""),
      populationScope: String(form.get("population") ?? ""),
      context: String(form.get("context") ?? ""),
      temporalBoundary: String(form.get("temporalBoundary") ?? ""),
      createdBy: null,
    }));
    target.reset();
  };

  const createObservationPlan = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    await act(async () => {
      const plan = await researchService.createObservationPlan({
        projectId,
        phenomenonId: String(form.get("phenomenonId") ?? ""),
        title: String(form.get("title") ?? ""),
        measurementMethod: String(form.get("measurementMethod") ?? ""),
        sampleFrame: String(form.get("sampleFrame") ?? ""),
        conditionsTemplate: String(form.get("conditions") ?? ""),
        inclusionRules: String(form.get("inclusionRules") ?? ""),
        createdBy: null,
      });
      const version = plan.versions[0];
      if (version) setSelectedPlanVersionId(version.id);
    });
    target.reset();
  };

  const createObservation = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const version = workspace?.observationPlans
      .flatMap((plan) => plan.versions)
      .find((item) => item.id === selectedPlanVersionId);
    const plan = workspace?.observationPlans.find((item) => item.id === version?.planId);
    if (!version || !plan) return;
    const target = event.currentTarget;
    const form = new FormData(target);
    const state = observationValueState(String(form.get("valueState") ?? "observed"));
    await act(() => researchService.createObservation({
      projectId,
      phenomenonId: plan.phenomenonId,
      planId: plan.id,
      planVersionId: version.id,
      datasetVersionReferenceId: selectedDatasetVersionId || null,
      measuredAtUs: Date.now() * 1000,
      valueDecimal: state === "observed" ? String(form.get("value") ?? "") : null,
      valueState: state,
      unit: state === "observed" ? String(form.get("unit") ?? "") : null,
      measurementMethod: version.measurementMethod,
      conditions: String(form.get("conditions") ?? ""),
      sampleIdentifier: String(form.get("sampleId") ?? "") || null,
      createdBy: null,
    }));
    target.reset();
  };

  const createDatasetVersion = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    const sourceVersion = sourceVersions.find(
      (source) => source.sourceVersionId === String(form.get("sourceVersionId") ?? ""),
    );
    if (!sourceVersion) return;
    await act(async () => {
      const dataset = await researchService.createDatasetVersionReference({
        projectId,
        title: String(form.get("title") ?? ""),
        versionLabel: String(form.get("version") ?? ""),
        externalUri: null,
        contentSha256: null,
        sourceVersionId: sourceVersion.sourceVersionId,
        createdBy: null,
      });
      setSelectedDatasetVersionId(dataset.id);
    });
    target.reset();
  };

  const createPattern = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    const observationId = String(form.get("observationId") ?? "");
    await act(async () => {
      await researchService.createPattern({
        projectId,
        title: String(form.get("title") ?? ""),
        statement: String(form.get("statement") ?? ""),
        analysisMethod: String(form.get("analysisMethod") ?? ""),
        uncertainty: String(form.get("uncertainty") ?? ""),
        observationIds: observationId ? [observationId] : [],
        datasetVersionReferenceIds: [],
        createdBy: null,
      });
    });
    target.reset();
  };

  const createIdea = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    const originKind = String(form.get("originKind") ?? "");
    const originId = String(form.get("originId") ?? "");
    await act(() => researchService.createIdea({
      projectId,
      title: String(form.get("title") ?? ""),
      content: String(form.get("content") ?? ""),
      phenomenonId: originKind === "phenomenon" ? originId : null,
      observationId: originKind === "observation" ? originId : null,
      patternId: originKind === "pattern" ? originId : null,
      questionId: originKind === "question" ? originId : null,
      createdBy: null,
    }));
    target.reset();
  };

  const createHypothesis = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    const questionId = String(form.get("questionId") ?? "");
    await act(() => researchService.createTypedHypothesis({
      projectId,
      researchQuestionId: questionId,
      parentHypothesisId: null,
      lineageRationale: null,
      statementH0: String(form.get("h0") ?? ""),
      statementH1: String(form.get("h1") ?? ""),
      populationScope: String(form.get("population") ?? ""),
      researchScope: String(form.get("scope") ?? ""),
      expectedDirection: hypothesisDirection(String(form.get("direction") ?? "non_directional")),
      expectedEffectDecimal: null,
      expectedEffectUnit: null,
      falsificationCriteria: String(form.get("falsification") ?? ""),
      assumptions: String(form.get("assumptions") ?? ""),
      variables: [
        {
          role: "independent",
          name: String(form.get("independentName") ?? ""),
          operationalDefinition: String(form.get("independentDefinition") ?? ""),
          valueKind: "continuous",
          unit: String(form.get("independentUnit") ?? ""),
          measurementMethod: String(form.get("independentMethod") ?? ""),
        },
        {
          role: "dependent",
          name: String(form.get("dependentName") ?? ""),
          operationalDefinition: String(form.get("dependentDefinition") ?? ""),
          valueKind: "continuous",
          unit: String(form.get("dependentUnit") ?? ""),
          measurementMethod: String(form.get("dependentMethod") ?? ""),
        },
      ],
      createdBy: null,
    }));
    target.reset();
  };

  const addHypothesisEvidence = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    const hypothesis = projectHypotheses.find((item) => item.id === selectedHypothesisId);
    const definition = hypothesis ? latest(hypothesis.definitions) : undefined;
    if (!definition) return;
    const [kind, id] = String(form.get("evidenceId") ?? "").split("|", 2);
    const polarityValue = String(form.get("polarity") ?? "supports");
    const polarity = polarityValue === "contradicts"
      ? "contradicts"
      : polarityValue === "qualifies" ? "qualifies" : "supports";
    await act(() => researchService.addHypothesisEvidence({
      hypothesisVersionId: definition.id,
      sourceClaimId: kind === "source_claim" ? id : null,
      observationId: kind === "observation" ? id : null,
      patternId: kind === "pattern" ? id : null,
      polarity,
      rationale: String(form.get("rationale") ?? ""),
      createdBy: null,
    }));
    target.reset();
  };

  const addCompetingHypotheses = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    await act(() => researchService.linkCompetingHypotheses({
      hypothesisId: String(form.get("hypothesisId") ?? ""),
      competingHypothesisId: String(form.get("competingId") ?? ""),
      rationale: String(form.get("rationale") ?? ""),
      createdBy: null,
    }));
    target.reset();
  };

  const addModelReference = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    await act(() => researchService.addHypothesisModelReference({
      hypothesisId: String(form.get("hypothesisId") ?? ""),
      modelId: String(form.get("modelId") ?? ""),
      modelVersionId: String(form.get("modelVersionId") ?? ""),
      label: String(form.get("label") ?? ""),
      rationale: String(form.get("rationale") ?? ""),
      createdBy: null,
    }));
    target.reset();
  };

  const refineQuestion = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const target = event.currentTarget;
    const form = new FormData(target);
    await act(() => researchService.refineQuestion(
      String(form.get("parentQuestionId") ?? ""),
      String(form.get("title") ?? ""),
      String(form.get("description") ?? ""),
      String(form.get("rationale") ?? ""),
      null,
    ));
    target.reset();
  };

  const createDesign = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const hypothesis = projectHypotheses.find((item) => item.id === selectedHypothesisId);
    const definition = hypothesis ? latest(hypothesis.definitions) : undefined;
    if (!hypothesis || !definition) return;
    const target = event.currentTarget;
    const form = new FormData(target);
    await act(() => researchService.createLinkedDesign({
        projectId,
        title: String(form.get("title") ?? ""),
        description: String(form.get("description") ?? ""),
        protocol: String(form.get("protocol") ?? ""),
        hypothesisVersionId: definition.id,
        relationshipKind: "tests",
        rationale: String(form.get("rationale") ?? ""),
        createdBy: null,
      }));
    target.reset();
  };

  const updateQuestionStatus = (question: ResearchQuestionDto, status: "review" | "accepted" | "deferred" | "rejected") =>
    act(() => researchService.transitionQuestion(question.id, status, null, "Updated in Research workspace."));

  if (isLoading && !workspace) {
    return <p className="mx-auto max-w-5xl py-16 text-center text-sm text-gray-400">Loading Research workspace...</p>;
  }
  if (!workspace) {
    return <p role="alert" className="mx-auto max-w-5xl py-8 text-sm text-rose-300">{error ?? "Research workspace is unavailable."}</p>;
  }

  const projectReviews = workspace.literatureReviews.filter((review) => review.projectId === projectId);
  const projectAssessments = workspace.priorAssessments.filter((item) => item.projectId === projectId);
  const planVersions = workspace.observationPlans
    .filter((plan) => plan.projectId === projectId)
    .flatMap((plan) => plan.versions);
  const sourceOptionLabel = (source: SourceVersionReferenceDto): string =>
    `${source.sourceTitle} — ${source.editionLabel ?? "retrieval"}${source.sourceLocationLabel ? ` / ${source.sourceLocationLabel}` : ""}`;

  return (
    <div className="mx-auto w-full max-w-5xl space-y-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="text-xl font-semibold text-white">Research domain</h2>
          <p className="mt-1 text-sm text-gray-400">
            Projects own plans; domains classify work. Observations, interpretations, hypotheses, and design records remain distinct.
          </p>
        </div>
        <button type="button" onClick={() => void refresh().catch(() => undefined)} className="rounded-lg border border-[#343741] p-2 text-gray-300 hover:bg-[#202229]" aria-label="Refresh Research workspace">
          <RefreshCw size={16} />
        </button>
      </div>

      {error && (
        <div role="alert" className="flex items-center gap-2 rounded-lg border border-rose-900/60 bg-rose-950/30 px-4 py-3 text-sm text-rose-200">
          <AlertCircle size={16} /> {error}
        </div>
      )}

      <section className="grid gap-5 lg:grid-cols-2">
        <form onSubmit={(event) => void createProject(event)} className="space-y-3 rounded-xl border border-[#292c34] bg-[#17191f] p-4">
          <h3 className="font-semibold text-white">Create research project</h3>
          <Field label="Title"><input className={inputClass} name="title" required maxLength={200} /></Field>
          <Field label="Aim"><textarea className={inputClass} name="aim" rows={2} /></Field>
          <fieldset>
            <legend className="mb-2 text-xs font-medium text-gray-400">Pathways</legend>
            <div className="space-y-1.5">
              {PATHWAYS.map((pathway) => (
                <label key={pathway.key} className="flex items-center gap-2 text-xs text-gray-300">
                  <input
                    type="checkbox"
                    checked={pathways.includes(pathway.key)}
                    onChange={(event) => setPathways((current) =>
                      event.target.checked
                        ? [...current, pathway.key]
                        : current.filter((value) => value !== pathway.key)
                    )}
                  />
                  {pathway.label}
                </label>
              ))}
            </div>
          </fieldset>
          <button className={buttonClass} disabled={pathways.length === 0}><Plus size={14} /> Create project</button>
        </form>

        <div className="space-y-4 rounded-xl border border-[#292c34] bg-[#17191f] p-4">
          <h3 className="font-semibold text-white">Project and reusable domains</h3>
          <ProjectSelect projects={workspace.projects} value={projectId} onChange={setProjectId} />
          {workspace.projects.length === 0 ? (
            <p className="text-xs text-amber-300">Create a project to begin. Existing legacy questions remain explicitly unassigned.</p>
          ) : (
            <div className="space-y-2">
              {workspace.projects.map((project) => (
                <button key={project.id} type="button" onClick={() => setProjectId(project.id)} className={`block w-full rounded-lg border p-3 text-left ${project.id === projectId ? "border-[#e8ad55]/60 bg-[#221f1a]" : "border-[#292c34] bg-[#101114]"}`}>
                  <span className="block text-sm font-medium text-white">{project.title}</span>
                  <span className="mt-1 block text-xs text-gray-400">{project.lifecycleStatus} · {project.pathwayKeys.join(", ")}</span>
                  <span className="mt-1 block text-[11px] text-gray-500">{project.domainIds.length} domain assignment(s)</span>
                </button>
              ))}
            </div>
          )}
          <form onSubmit={(event) => void createDomain(event)} className="space-y-2 border-t border-[#292c34] pt-3">
            <Field label="Domain name"><input className={inputClass} name="name" required maxLength={200} /></Field>
            <Field label="Description"><input className={inputClass} name="description" /></Field>
            <button className={buttonClass} disabled={!projectId}><Plus size={14} /> Create and assign domain</button>
          </form>
          <div className="flex flex-wrap gap-2">
            {workspace.domains.map((domain) => (
              <span key={domain.id} className="rounded-full border border-[#343741] px-2.5 py-1 text-xs text-gray-300">
                {domain.name}{workspace.projects.find((project) => project.id === projectId)?.domainIds.includes(domain.id) ? " · assigned" : ""}
              </span>
            ))}
          </div>
          <form onSubmit={(event) => void adoptFrameworkVersion(event)} className="space-y-2 border-t border-[#292c34] pt-3">
            <h4 className="text-xs font-semibold text-gray-300">Adopt an external knowledge version</h4>
            <Field label="Published framework version"><select className={inputClass} name="frameworkVersionId" required><option value="">Select a published version</option>{workspace.frameworkVersions.map((version) => <option key={version.frameworkVersionId} value={version.frameworkVersionId}>{version.title} · v{version.versionNo}</option>)}</select></Field>
            <Field label="Use context"><input className={inputClass} name="context" required /></Field>
            <button className={buttonClass} disabled={!projectId || workspace.frameworkVersions.length === 0}>Adopt immutable version</button>
            {workspace.adoptions.filter((item) => item.projectId === projectId).map((item) => (
              <p key={item.id} className="text-[11px] text-gray-500">{item.title} v{item.versionNo} · {item.useContext}</p>
            ))}
          </form>
        </div>
      </section>

      {projectId && (
        <>
          <section className="grid gap-5 lg:grid-cols-2">
            <form onSubmit={(event) => void createPriorAssessment(event)} className="space-y-3 rounded-xl border border-[#292c34] bg-[#17191f] p-4">
              <h3 className="font-semibold text-white">Prior-knowledge assessment</h3>
              <p className="text-xs text-gray-500">A reviewable assessment can support research-gap acceptance.</p>
              <Field label="Title"><input className={inputClass} name="title" required /></Field>
              <Field label="Assessment"><textarea className={inputClass} name="assessment" rows={3} required /></Field>
              <button className={buttonClass}><BookOpenCheck size={14} /> Save as reviewed assessment</button>
            </form>

            <form onSubmit={(event) => void createLiteratureReview(event)} className="space-y-3 rounded-xl border border-[#292c34] bg-[#17191f] p-4">
              <h3 className="font-semibold text-white">Literature review</h3>
              <p className="text-xs text-gray-500">A reviewed source-version citation is required before this review can support a gap.</p>
              <Field label="Review title"><input className={inputClass} name="title" required /></Field>
              <Field label="Search scope"><textarea className={inputClass} name="scope" rows={2} required /></Field>
              <Field label="Exact source version and location">
                <select className={inputClass} value={selectedSourceKey} onChange={(event) => setSelectedSourceKey(event.target.value)} required>
                  <option value="">Select a source version</option>
                  {sourceVersions.map((source, index) => (
                    <option key={`${sourceKey(source)}-${index}`} value={sourceKey(source)}>{sourceOptionLabel(source)}</option>
                  ))}
                </select>
              </Field>
              {selectedSource && (
                <p className="rounded bg-[#101114] p-2 text-[11px] text-gray-400">
                  Source {selectedSource.sourceId} · version {selectedSource.sourceVersionId}
                  {selectedSource.sourceLocationId ? ` · location ${selectedSource.sourceLocationId}` : " · no specific location"}
                  {selectedSource.sourceClaimId ? ` · claim ${selectedSource.sourceClaimId}` : ""}
                </p>
              )}
              <Field label="Citation note"><input className={inputClass} name="citationNote" /></Field>
              <button className={buttonClass} disabled={!selectedSource}><BookOpenCheck size={14} /> Add citation and review</button>
            </form>
            {projectReviews.map((review) => (
              <article key={review.id} className="rounded-lg border border-[#292c34] bg-[#101114] p-3">
                <h4 className="text-sm text-white">{review.title} · {review.lifecycleStatus}</h4>
                <p className="mt-1 text-xs text-gray-500">Search scope: {review.searchScope}</p>
                {review.sources.map((citation) => (
                  <p key={citation.id} className="mt-1 text-[11px] text-gray-400">
                    {citation.sourceId} · exact version {citation.sourceVersionId}
                    {citation.sourceLocationId ? ` · location ${citation.sourceLocationId}` : " · location unspecified"}
                    {citation.sourceClaimId ? ` · source claim ${citation.sourceClaimId}` : ""}
                    {" · "}{citation.citationNote || "no note"} · <Link to="/sources" className="text-amber-300 underline">Open source workspace</Link>
                  </p>
                ))}
              </article>
            ))}
          </section>

          <section className="space-y-4 rounded-xl border border-[#292c34] bg-[#17191f] p-4">
            <h3 className="font-semibold text-white">Research gaps and evidence</h3>
            <form onSubmit={(event) => void createGap(event)} className="grid gap-3 md:grid-cols-2">
              <Field label="Gap title"><input className={inputClass} name="title" required /></Field>
              <Field label="Gap statement"><input className={inputClass} name="statement" required /></Field>
              <Field label="Domain (optional)"><select className={inputClass} name="domainId"><option value="">No domain</option>{projectDomains.map((domain) => <option key={domain.id} value={domain.id}>{domain.name}</option>)}</select></Field>
              <Field label="Evidence type">
                <select className={inputClass} name="evidenceKind">
                  <option value="prior">Prior-knowledge assessment</option>
                  <option value="literature">Literature review</option>
                </select>
              </Field>
              <Field label="Reviewed evidence">
                <select className={inputClass} name="evidenceId">
                  <option value="">Create without evidence (cannot accept yet)</option>
                  <optgroup label="Prior-knowledge assessments">
                    {projectAssessments.filter((item) => item.lifecycleStatus === "reviewed").map((item) => <option key={item.id} value={item.id}>{item.title} · {isoDate(item.createdAtUs)}</option>)}
                  </optgroup>
                  <optgroup label="Literature reviews">
                    {projectReviews.filter((item) => item.lifecycleStatus === "reviewed").map((item) => <option key={item.id} value={item.id}>{item.title} · {isoDate(item.createdAtUs)}</option>)}
                  </optgroup>
                </select>
              </Field>
              <div><button className={buttonClass}><Plus size={14} /> Create gap</button></div>
            </form>
            <div className="space-y-2">
              {workspace.gaps.filter((gap) => gap.projectId === projectId).map((gap) => (
                <article key={gap.id} className="rounded-lg border border-[#292c34] bg-[#101114] p-3">
                  <div className="flex flex-wrap items-start justify-between gap-2">
                    <div><h4 className="text-sm font-medium text-white">{gap.title}</h4><p className="mt-1 text-xs text-gray-400">{gap.statement}</p></div>
                    <span className="rounded bg-[#242630] px-2 py-1 text-[11px] text-gray-300">{gap.lifecycleStatus}</span>
                  </div>
                  <p className="mt-2 text-[11px] text-gray-500">
                    Evidence: {gap.literatureReviewIds.length} literature review(s), {gap.priorAssessmentIds.length} assessment(s), {gap.hasReviewedException ? "reviewed exception" : "no exception"}
                  </p>
                  <form onSubmit={(event) => { event.preventDefault(); void addGapEvidence(gap, event.currentTarget); }} className="mt-2 flex flex-wrap gap-2">
                    <select className="min-w-[170px] rounded border border-[#343741] bg-[#17191f] px-2 py-1 text-xs text-white" name="evidenceKind">
                      <option value="prior">Prior assessment</option><option value="literature">Literature review</option>
                    </select>
                    <select className="min-w-[200px] flex-1 rounded border border-[#343741] bg-[#17191f] px-2 py-1 text-xs text-white" name="evidenceId">
                      <option value="">Add reviewed evidence…</option>
                      {projectAssessments.filter((item) => item.lifecycleStatus === "reviewed").map((item) => <option key={item.id} value={item.id}>{item.title} · {isoDate(item.createdAtUs)}</option>)}
                      {projectReviews.filter((item) => item.lifecycleStatus === "reviewed").map((item) => <option key={item.id} value={item.id}>{item.title} · {isoDate(item.createdAtUs)}</option>)}
                    </select>
                    <button className="rounded border border-[#343741] px-2 py-1 text-xs text-gray-300">Attach evidence</button>
                  </form>
                  <div className="mt-2 flex gap-2">
                    {gap.lifecycleStatus === "candidate" && <button type="button" onClick={() => void act(() => researchService.transitionGap(gap.id, "assessed", null))} className="rounded border border-[#343741] px-2 py-1 text-xs text-gray-300">Mark assessed</button>}
                    {gap.lifecycleStatus === "assessed" && <button type="button" onClick={() => void act(() => researchService.transitionGap(gap.id, "accepted", null))} className="rounded border border-emerald-800 px-2 py-1 text-xs text-emerald-300">Accept gap</button>}
                  </div>
                </article>
              ))}
            </div>
            <form onSubmit={(event) => {
              event.preventDefault();
              const form = new FormData(event.currentTarget);
              void act(() => researchService.reviewGapException({
                gapId: String(form.get("gapId") ?? ""),
                rationale: String(form.get("rationale") ?? ""),
                reviewerId: null,
                reviewDecision: "accepted",
              }));
              event.currentTarget.reset();
            }} className="grid gap-2 border-t border-[#292c34] pt-3 md:grid-cols-[1fr_2fr_auto]">
              <Field label="Reviewed exception for"><select className={inputClass} name="gapId" required><option value="">Select a gap</option>{workspace.gaps.filter((gap) => gap.projectId === projectId).map((gap) => <option key={gap.id} value={gap.id}>{gap.title}</option>)}</select></Field>
              <Field label="Rationale"><input className={inputClass} name="rationale" required /></Field>
              <button className={`${buttonClass} self-end`}>Record reviewed exception</button>
            </form>
          </section>

          <section className="grid gap-5 lg:grid-cols-2">
            <div className="space-y-3 rounded-xl border border-[#292c34] bg-[#17191f] p-4">
              <h3 className="font-semibold text-white">Question lifecycle</h3>
              <form onSubmit={(event) => void createQuestion(event)} className="space-y-3">
                <ProjectSelect projects={workspace.projects} value={questionProjectId} onChange={setQuestionProjectId} />
                <Field label="Accepted gap (optional)">
                  <select className={inputClass} name="gapId">
                    <option value="">Unlinked draft question</option>
                    {(workspace.gaps.filter((gap) => gap.projectId === questionProjectId && gap.lifecycleStatus === "accepted")).map((gap) => <option key={gap.id} value={gap.id}>{gap.title}</option>)}
                  </select>
                </Field>
                <Field label="Domain (optional)"><select className={inputClass} name="domainId"><option value="">No domain</option>{questionProjectDomains.map((domain) => <option key={domain.id} value={domain.id}>{domain.name}</option>)}</select></Field>
                <Field label="Question"><input className={inputClass} name="title" required /></Field>
                <Field label="Description"><textarea className={inputClass} name="description" rows={2} /></Field>
                <Field label="Rationale"><input className={inputClass} name="rationale" /></Field>
                <button className={buttonClass}><Plus size={14} /> Create draft question</button>
              </form>
              <div className="space-y-2 border-t border-[#292c34] pt-3">
                {workspace.questions.filter((question) => question.projectId === questionProjectId || (!question.projectId && question.lifecycleStatus === "legacy_unassessed")).map((question) => (
                  <article key={question.id} className="rounded-lg border border-[#292c34] bg-[#101114] p-3">
                    <div className="flex items-center justify-between gap-2"><h4 className="text-sm text-white">{question.title}</h4><span className="text-[11px] text-gray-400">{question.lifecycleStatus} · {question.ownershipState}</span></div>
                    <p className="mt-1 text-xs text-gray-400">{question.description}</p>
                    {question.migrationNote && <p className="mt-1 text-[11px] text-amber-300">{question.migrationNote}</p>}
                    <p className="mt-1 text-[11px] text-gray-500">Question ID {question.id}{question.researchGapId ? ` · gap ${question.researchGapId}` : " · no gap link"}</p>
                    {question.projectId && ["draft", "review", "deferred"].includes(question.lifecycleStatus) && (
                      <div className="mt-2 flex flex-wrap gap-2">
                        {question.lifecycleStatus === "draft" && <button type="button" onClick={() => void updateQuestionStatus(question, "review")} className="rounded border border-[#343741] px-2 py-1 text-xs text-gray-300">Submit for review</button>}
                        {(question.lifecycleStatus === "review" || question.lifecycleStatus === "deferred") && <button type="button" onClick={() => void updateQuestionStatus(question, "accepted")} className="rounded border border-emerald-800 px-2 py-1 text-xs text-emerald-300">Accept question</button>}
                        <button type="button" onClick={() => void updateQuestionStatus(question, "deferred")} className="rounded border border-[#343741] px-2 py-1 text-xs text-gray-400">Defer</button>
                        <button type="button" onClick={() => void updateQuestionStatus(question, "rejected")} className="rounded border border-rose-900 px-2 py-1 text-xs text-rose-300">Reject</button>
                      </div>
                    )}
                  </article>
                ))}
              </div>
              <form onSubmit={(event) => void refineQuestion(event)} className="space-y-2 border-t border-[#292c34] pt-3">
                <h4 className="text-xs font-semibold text-gray-300">Refine an accepted question</h4>
                <Field label="Parent question"><select className={inputClass} name="parentQuestionId" required><option value="">Select accepted parent</option>{projectQuestions.filter((question) => question.lifecycleStatus === "accepted").map((question) => <option key={question.id} value={question.id}>{question.title}</option>)}</select></Field>
                <Field label="Derived question"><input className={inputClass} name="title" required /></Field>
                <Field label="Description"><input className={inputClass} name="description" /></Field>
                <Field label="Refinement rationale"><input className={inputClass} name="rationale" required /></Field>
                <button className={buttonClass} disabled={!projectQuestions.some((question) => question.lifecycleStatus === "accepted")}>Create refined child question</button>
              </form>
            </div>

            <div className="space-y-3 rounded-xl border border-[#292c34] bg-[#17191f] p-4">
              <h3 className="font-semibold text-white">Phenomena and observation plans</h3>
              <form onSubmit={(event) => void createPhenomenon(event)} className="space-y-2">
                <Field label="Phenomenon"><input className={inputClass} name="title" required /></Field>
                <Field label="Description"><textarea className={inputClass} name="description" required rows={2} /></Field>
                <Field label="Population / scope"><input className={inputClass} name="population" required /></Field>
                <Field label="Domain (optional)"><select className={inputClass} name="domainId"><option value="">No domain</option>{projectDomains.map((domain) => <option key={domain.id} value={domain.id}>{domain.name}</option>)}</select></Field>
                <Field label="Conditions and context"><input className={inputClass} name="context" /></Field>
                <Field label="Temporal boundary"><input className={inputClass} name="temporalBoundary" /></Field>
                <button className={buttonClass}><Plus size={14} /> Register phenomenon</button>
              </form>
              <form onSubmit={(event) => void createObservationPlan(event)} className="space-y-2 border-t border-[#292c34] pt-3">
                <Field label="Phenomenon"><select className={inputClass} name="phenomenonId" required><option value="">Select phenomenon</option>{projectPhenomena.map((item) => <option key={item.id} value={item.id}>{item.title}</option>)}</select></Field>
                <Field label="Plan title"><input className={inputClass} name="title" required /></Field>
                <Field label="Measurement method"><input className={inputClass} name="measurementMethod" required /></Field>
                <Field label="Sample frame"><input className={inputClass} name="sampleFrame" required /></Field>
                <Field label="Conditions template"><input className={inputClass} name="conditions" /></Field>
                <Field label="Inclusion rules"><input className={inputClass} name="inclusionRules" /></Field>
                <button className={buttonClass}><Plus size={14} /> Create plan version</button>
              </form>
              <div className="space-y-2 border-t border-[#292c34] pt-3">
                {planVersions.map((version) => (
                  <div key={version.id} className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-[#292c34] bg-[#101114] p-2">
                    <button type="button" onClick={() => setSelectedPlanVersionId(version.id)} className="text-left text-xs text-gray-300">
                      {workspace.observationPlans.find((plan) => plan.id === version.planId)?.title} · v{version.versionNo} · {version.lifecycleStatus}
                    </button>
                    {version.lifecycleStatus === "draft" && <button type="button" onClick={() => void act(() => researchService.reviewObservationPlan(version.id, false, null))} className="rounded border border-[#343741] px-2 py-1 text-xs text-gray-300">Review plan</button>}
                  </div>
                ))}
              </div>
              <form onSubmit={(event) => void createDatasetVersion(event)} className="space-y-2 border-t border-[#292c34] pt-3">
                <p className="text-xs font-semibold text-gray-300">Immutable dataset-version reference</p>
                <Field label="Dataset title"><input className={inputClass} name="title" required /></Field>
                <Field label="Dataset version"><input className={inputClass} name="version" required /></Field>
                <Field label="Source version"><select className={inputClass} name="sourceVersionId" required><option value="">Select an acquired source version</option>{sourceVersions.map((source, index) => <option key={`${source.sourceVersionId}-${index}`} value={source.sourceVersionId}>{sourceOptionLabel(source)} · {source.sourceVersionId}</option>)}</select></Field>
                <button className={buttonClass} disabled={sourceVersions.length === 0}><Plus size={14} /> Save exact dataset version</button>
              </form>
              <form onSubmit={(event) => void createObservation(event)} className="space-y-2 border-t border-[#292c34] pt-3">
                <Field label="Reviewed plan version"><select className={inputClass} value={selectedPlanVersionId} onChange={(event) => setSelectedPlanVersionId(event.target.value)} required><option value="">Select reviewed plan</option>{planVersions.filter((version) => version.lifecycleStatus === "reviewed" || version.lifecycleStatus === "preregistered").map((version) => <option key={version.id} value={version.id}>{version.id} · {version.lifecycleStatus}</option>)}</select></Field>
                <Field label="Dataset version (optional)"><select className={inputClass} value={selectedDatasetVersionId} onChange={(event) => setSelectedDatasetVersionId(event.target.value)}><option value="">No external dataset reference</option>{workspace.datasetVersions.filter((dataset) => dataset.projectId === projectId).map((dataset) => <option key={dataset.id} value={dataset.id}>{dataset.title} · {dataset.versionLabel} · source version {dataset.sourceVersionId}</option>)}</select></Field>
                <Field label="Value state"><select className={inputClass} name="valueState"><option value="observed">Observed value</option><option value="not_measured">Not measured</option><option value="not_recorded">Not recorded</option><option value="unknown">Unknown</option><option value="not_applicable">Not applicable</option><option value="withheld">Withheld</option><option value="invalid">Invalid</option></select></Field>
                <div className="grid grid-cols-2 gap-2"><Field label="Value"><input className={inputClass} name="value" /></Field><Field label="Unit"><input className={inputClass} name="unit" /></Field></div>
                <Field label="Observed conditions"><input className={inputClass} name="conditions" required /></Field>
                <Field label="Sample identifier"><input className={inputClass} name="sampleId" /></Field>
                <button className={buttonClass} disabled={!selectedPlanVersionId}><Plus size={14} /> Record observation</button>
              </form>
            </div>
          </section>

          <section className="grid gap-5 lg:grid-cols-2">
            <div className="space-y-3 rounded-xl border border-[#292c34] bg-[#17191f] p-4">
              <h3 className="font-semibold text-white">Patterns and ideas</h3>
              <form onSubmit={(event) => void createPattern(event)} className="space-y-2">
                <Field label="Pattern title"><input className={inputClass} name="title" required /></Field>
                <Field label="Interpretation"><textarea className={inputClass} name="statement" required rows={2} /></Field>
                <Field label="Declared observation"><select className={inputClass} name="observationId"><option value="">Select observation</option>{projectObservations.map((item) => <option key={item.id} value={item.id}>{item.id} · {item.valueState}</option>)}</select></Field>
                <Field label="Analysis method"><input className={inputClass} name="analysisMethod" required /></Field>
                <Field label="Uncertainty"><input className={inputClass} name="uncertainty" /></Field>
                <button className={buttonClass}><Plus size={14} /> Record interpreted pattern</button>
              </form>
              <form onSubmit={(event) => void createIdea(event)} className="space-y-2 border-t border-[#292c34] pt-3">
                <Field label="Idea title"><input className={inputClass} name="title" required /></Field>
                <Field label="Proposal"><textarea className={inputClass} name="content" required rows={2} /></Field>
                <Field label="Typed origin">
                  <select className={inputClass} name="originKind"><option value="pattern">Pattern</option><option value="question">Question</option><option value="observation">Observation</option><option value="phenomenon">Phenomenon</option></select>
                </Field>
                <Field label="Origin record">
                  <select className={inputClass} name="originId" required>
                    <option value="">Select origin</option>
                    {workspace.patterns.filter((item) => item.projectId === projectId).map((item) => <option key={`p-${item.id}`} value={item.id}>{item.title}</option>)}
                    {projectQuestions.map((item) => <option key={`q-${item.id}`} value={item.id}>{item.title}</option>)}
                    {projectObservations.map((item) => <option key={`o-${item.id}`} value={item.id}>Observation {item.id}</option>)}
                    {projectPhenomena.map((item) => <option key={`f-${item.id}`} value={item.id}>{item.title}</option>)}
                  </select>
                </Field>
                <button className={buttonClass}><Plus size={14} /> Capture unvalidated idea</button>
              </form>
              <div className="space-y-2 border-t border-[#292c34] pt-3">
                {workspace.patterns.filter((item) => item.projectId === projectId).map((pattern) => (
                  <article key={pattern.id} className="rounded-lg border border-[#292c34] bg-[#101114] p-3">
                    <h4 className="text-sm text-white">{pattern.title} <span className="text-xs text-gray-500">· {pattern.lifecycleStatus}</span></h4>
                    <p className="mt-1 text-xs text-gray-400">{pattern.statement}</p>
                    <p className="mt-1 text-[11px] text-gray-500">Interpretation of observations: {pattern.observationIds.join(", ") || "none"}</p>
                  </article>
                ))}
                {workspace.ideas.filter((item) => item.projectId === projectId).map((idea) => (
                  <p key={idea.id} className="rounded-lg border border-[#292c34] bg-[#101114] p-3 text-xs text-gray-300">
                    Idea · {idea.title} · unvalidated · origin {idea.patternId ?? idea.questionId ?? idea.observationId ?? idea.phenomenonId}
                  </p>
                ))}
              </div>
            </div>

            <div className="space-y-3 rounded-xl border border-[#292c34] bg-[#17191f] p-4">
              <h3 className="font-semibold text-white">Typed hypotheses and design-only records</h3>
              <form onSubmit={(event) => void createHypothesis(event)} className="space-y-2">
                <Field label="Accepted research question"><select className={inputClass} name="questionId" required><option value="">Select accepted question</option>{projectQuestions.filter((question) => question.lifecycleStatus === "accepted").map((question) => <option key={question.id} value={question.id}>{question.title}</option>)}</select></Field>
                <Field label="H0 — null hypothesis"><textarea className={inputClass} name="h0" required rows={2} /></Field>
                <Field label="H1 — alternative hypothesis"><textarea className={inputClass} name="h1" required rows={2} /></Field>
                <div className="grid grid-cols-2 gap-2"><Field label="Population / scope"><input className={inputClass} name="population" required /></Field><Field label="Research scope"><input className={inputClass} name="scope" required /></Field></div>
                <Field label="Expected direction"><select className={inputClass} name="direction"><option value="non_directional">Non-directional</option><option value="positive">Positive</option><option value="negative">Negative</option><option value="equivalence">Equivalence</option><option value="other">Other</option></select></Field>
                <div className="rounded-lg border border-[#292c34] p-3">
                  <p className="mb-2 text-xs font-semibold text-gray-300">Independent variable — operational definition</p>
                  <div className="grid gap-2 sm:grid-cols-2"><Field label="Name"><input className={inputClass} name="independentName" required /></Field><Field label="Unit"><input className={inputClass} name="independentUnit" required /></Field></div>
                  <Field label="Definition"><input className={inputClass} name="independentDefinition" required /></Field>
                  <Field label="Measurement method"><input className={inputClass} name="independentMethod" required /></Field>
                </div>
                <div className="rounded-lg border border-[#292c34] p-3">
                  <p className="mb-2 text-xs font-semibold text-gray-300">Dependent variable — operational definition</p>
                  <div className="grid gap-2 sm:grid-cols-2"><Field label="Name"><input className={inputClass} name="dependentName" required /></Field><Field label="Unit"><input className={inputClass} name="dependentUnit" required /></Field></div>
                  <Field label="Definition"><input className={inputClass} name="dependentDefinition" required /></Field>
                  <Field label="Measurement method"><input className={inputClass} name="dependentMethod" required /></Field>
                </div>
                <Field label="Falsification criteria"><textarea className={inputClass} name="falsification" required rows={2} /></Field>
                <Field label="Assumptions"><textarea className={inputClass} name="assumptions" rows={2} /></Field>
                <button className={buttonClass}><Plus size={14} /> Create typed hypothesis</button>
              </form>
              <div className="space-y-2 border-t border-[#292c34] pt-3">
                {projectHypotheses.map((hypothesis) => {
                  const definition = latest(hypothesis.definitions);
                  return (
                    <article key={hypothesis.id} className="rounded-lg border border-[#292c34] bg-[#101114] p-3">
                      <div className="flex flex-wrap items-center justify-between gap-2">
                        <h4 className="text-sm text-white">{definition?.statementH1 ?? "Legacy hypothesis — no typed definition"}</h4>
                        <span className="text-[11px] text-amber-300">{hypothesis.lifecycleStatus}</span>
                      </div>
                      {definition && (
                        <>
                          <p className="mt-1 text-xs text-gray-400">H0: {definition.statementH0}</p>
                          <p className="mt-1 text-[11px] text-gray-500">Question {hypothesis.researchQuestionId} · created {isoDate(hypothesis.createdAtUs)} · definition v{definition.versionNo} · {definition.lifecycleStatus} · {definition.variables.length} operational variable(s)</p>
                          <p className="mt-1 text-[11px] text-gray-500">Falsification: {definition.falsificationCriteria}</p>
                          {hypothesis.evidence.map((evidence, index) => (
                            <p key={`${evidence.targetId}-${index}`} className="mt-1 text-[11px] text-gray-400">
                              {evidence.polarity} {evidence.targetKind} {evidence.targetId}
                              {evidence.sourceVersionId ? ` · source ${evidence.sourceId} version ${evidence.sourceVersionId} location ${evidence.sourceLocationId ?? "unspecified"}` : ""}
                              {" · "}{evidence.rationale}
                            </p>
                          ))}
                        </>
                      )}
                      {hypothesis.lifecycleStatus === "draft" && <button type="button" onClick={() => void act(() => researchService.transitionHypothesis(hypothesis.id, "review", null))} className="mt-2 rounded border border-[#343741] px-2 py-1 text-xs text-gray-300">Submit typed definition for review</button>}
                      {hypothesis.lifecycleStatus === "review" && <button type="button" onClick={() => void act(() => researchService.transitionHypothesis(hypothesis.id, "preregistered", null))} className="mt-2 rounded border border-emerald-800 px-2 py-1 text-xs text-emerald-300">Preregister definition</button>}
                    </article>
                  );
                })}
              </div>
              <form onSubmit={(event) => void addHypothesisEvidence(event)} className="space-y-2 border-t border-[#292c34] pt-3">
                <h4 className="text-xs font-semibold text-gray-300">Attach typed evidence to the current definition</h4>
                <Field label="Hypothesis"><select className={inputClass} value={selectedHypothesisId} onChange={(event) => setSelectedHypothesisId(event.target.value)} required><option value="">Select a typed hypothesis</option>{projectHypotheses.filter((item) => item.definitions.length > 0).map((item) => <option key={item.id} value={item.id}>{latest(item.definitions)?.statementH1}</option>)}</select></Field>
                <Field label="Evidence record"><select className={inputClass} name="evidenceId" required>
                  <option value="">Select source claim, observation, or pattern</option>
                  {sourceVersions.filter((source) => source.sourceClaimId).map((source, index) => <option key={`${source.sourceClaimId}-${index}`} value={`source_claim|${source.sourceClaimId}`}>Source claim {source.sourceClaimId} · source version {source.sourceVersionId}</option>)}
                  {projectObservations.map((observation) => <option key={observation.id} value={`observation|${observation.id}`}>Observation {observation.id}</option>)}
                  {workspace.patterns.filter((item) => item.projectId === projectId).map((pattern) => <option key={pattern.id} value={`pattern|${pattern.id}`}>Pattern {pattern.title}</option>)}
                </select></Field>
                <Field label="Evidence relationship"><select className={inputClass} name="polarity"><option value="supports">Supports</option><option value="contradicts">Contradicts</option><option value="qualifies">Qualifies</option></select></Field>
                <Field label="Rationale"><input className={inputClass} name="rationale" required /></Field>
                <button className={buttonClass} disabled={!projectHypotheses.some((item) => item.definitions.length > 0)}>Attach evidence</button>
              </form>
              <form onSubmit={(event) => void addCompetingHypotheses(event)} className="space-y-2 border-t border-[#292c34] pt-3">
                <h4 className="text-xs font-semibold text-gray-300">Preserve competing hypotheses</h4>
                <div className="grid gap-2 sm:grid-cols-2">
                  <Field label="Hypothesis"><select className={inputClass} name="hypothesisId" required><option value="">Select hypothesis</option>{projectHypotheses.filter((item) => item.definitions.length > 0).map((item) => <option key={item.id} value={item.id}>{latest(item.definitions)?.statementH1}</option>)}</select></Field>
                  <Field label="Competing hypothesis"><select className={inputClass} name="competingId" required><option value="">Select alternative</option>{projectHypotheses.filter((item) => item.definitions.length > 0).map((item) => <option key={item.id} value={item.id}>{latest(item.definitions)?.statementH1}</option>)}</select></Field>
                </div>
                <Field label="Competition rationale"><input className={inputClass} name="rationale" required /></Field>
                <button className={buttonClass} disabled={projectHypotheses.filter((item) => item.definitions.length > 0).length < 2}>Link as competitors</button>
              </form>
              <form onSubmit={(event) => void addModelReference(event)} className="space-y-2 border-t border-[#292c34] pt-3">
                <h4 className="text-xs font-semibold text-gray-300">Model and version reference</h4>
                <Field label="Hypothesis"><select className={inputClass} name="hypothesisId" required><option value="">Select hypothesis</option>{projectHypotheses.map((item) => <option key={item.id} value={item.id}>{item.id}</option>)}</select></Field>
                <div className="grid grid-cols-2 gap-2"><Field label="Model ID"><input className={inputClass} name="modelId" required /></Field><Field label="Model version ID"><input className={inputClass} name="modelVersionId" required /></Field></div>
                <Field label="Label"><input className={inputClass} name="label" required /></Field>
                <Field label="Rationale"><input className={inputClass} name="rationale" required /></Field>
                <p className="text-[11px] text-amber-300">Model identity remains unresolved until linked to a verified model record.</p>
                <button className={buttonClass}>Add unresolved model reference</button>
              </form>
              <form onSubmit={(event) => void createDesign(event)} className="space-y-2 border-t border-[#292c34] pt-3">
                <Field label="Hypothesis version to link"><select className={inputClass} value={selectedHypothesisId} onChange={(event) => setSelectedHypothesisId(event.target.value)} required><option value="">Select hypothesis</option>{projectHypotheses.filter((item) => item.definitions.length > 0).map((item) => <option key={item.id} value={item.id}>{latest(item.definitions)?.statementH1}</option>)}</select></Field>
                <Field label="Design title"><input className={inputClass} name="title" required /></Field>
                <Field label="Description"><textarea className={inputClass} name="description" required rows={2} /></Field>
                <Field label="Design protocol (not execution)"><textarea className={inputClass} name="protocol" required rows={2} /></Field>
                <Field label="Why this design relates to the hypothesis"><input className={inputClass} name="rationale" required /></Field>
                <button className={buttonClass} disabled={!selectedHypothesisId}><Plus size={14} /> Create design and link hypothesis</button>
              </form>
              <div className="space-y-2 border-t border-[#292c34] pt-3">
                {workspace.designVersions.filter((design) => design.projectId === projectId).map((design) => (
                  <div key={design.id} className="rounded-lg border border-[#292c34] bg-[#101114] p-3 text-xs text-gray-300">
                    <p>Design v{design.versionNo} · {design.title} · {design.lifecycleStatus} · no execution data in this phase</p>
                    <div className="mt-2 flex gap-2">
                      {design.lifecycleStatus === "draft" && <button type="button" onClick={() => void act(() => researchService.transitionDesign(design.id, "review", null))} className="rounded border border-[#343741] px-2 py-1 text-xs">Review design</button>}
                      {design.lifecycleStatus === "review" && <button type="button" onClick={() => void act(() => researchService.transitionDesign(design.id, "preregistered", null))} className="rounded border border-emerald-800 px-2 py-1 text-xs text-emerald-300">Preregister design</button>}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </section>
        </>
      )}
    </div>
  );
};

function sourceKey(source: SourceVersionReferenceDto): string {
  return `${source.sourceId}|${source.sourceVersionId}|${source.sourceLocationId ?? ""}`;
}
