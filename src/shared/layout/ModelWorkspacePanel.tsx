import React, { useCallback, useEffect, useMemo, useState } from "react";
import { AlertCircle, RefreshCw } from "lucide-react";
import { analysisService } from "@/mind/services/analysisService";
import { experimentService } from "@/mind/services/experimentService";
import { modelService } from "@/mind/services/modelService";
import type {
  AnalysisWorkspaceDto,
  CreateModelVersionReq,
  ExperimentWorkspaceDto,
  ModelArtifactRef,
  ModelDatasetUseReq,
  ModelDependencyReq,
  ModelEvidencePolarity,
  ModelExperimentRelationReq,
  ModelImpactReq,
  ModelParameterReq,
  ModelRelationReq,
  ModelWorkspaceDto,
} from "@/types";

const fieldClass =
  "w-full rounded-lg border border-[#343741] bg-[#101114] px-3 py-2 text-sm text-white outline-none focus:border-[#e8ad55]";
const buttonClass =
  "rounded-lg border border-[#343741] px-3 py-2 text-xs font-medium text-gray-200 hover:bg-[#202229] disabled:opacity-40";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function isModelParameter(value: unknown): value is ModelParameterReq {
  return isRecord(value)
    && typeof value.name === "string"
    && typeof value.value === "string"
    && ["decimal", "integer", "boolean", "string", "json"].includes(String(value.valueKind))
    && isNullableString(value.unit)
    && isNullableString(value.lowerBound)
    && isNullableString(value.upperBound)
    && isNullableString(value.defaultValue)
    && typeof value.role === "string"
    && typeof value.tuneable === "boolean";
}

function isDependency(value: unknown): value is ModelDependencyReq {
  return isRecord(value)
    && typeof value.name === "string"
    && typeof value.version === "string"
    && isNullableString(value.sourceUri)
    && isNullableString(value.checksumSha256);
}

function isArtifact(value: unknown): value is CreateModelVersionReq["artifacts"][number] {
  return isRecord(value)
    && typeof value.artifactKind === "string"
    && typeof value.title === "string"
    && typeof value.artifactUri === "string"
    && typeof value.checksumSha256 === "string"
    && isNullableString(value.mediaType)
    && (value.byteLength === null || typeof value.byteLength === "number");
}

function isRelation(value: unknown): value is ModelRelationReq {
  return isRecord(value)
    && typeof value.targetId === "string"
    && ["contributes_to", "contradicts", "qualifies"].includes(String(value.relationKind))
    && typeof value.rationale === "string";
}

function isExperimentRelation(value: unknown): value is ModelExperimentRelationReq {
  return isRecord(value)
    && typeof value.designVersionId === "string"
    && isNullableString(value.experimentRunId)
    && ["contributes_to", "tested_in"].includes(String(value.relationKind))
    && typeof value.rationale === "string";
}

function isDatasetUse(value: unknown): value is ModelDatasetUseReq {
  return isRecord(value)
    && typeof value.datasetVersionId === "string"
    && ["training", "validation", "test", "exploratory", "prospective", "forward"].includes(String(value.role))
    && isNullableString(value.splitGroupId)
    && typeof value.rationale === "string";
}

function isImpact(value: unknown): value is ModelImpactReq {
  if (!isRecord(value) || !isRecord(value.target) || typeof value.rationale !== "string") return false;
  const target = value.target;
  const validTarget = (target.kind === "source_claim" && typeof target.claimId === "string")
    || (target.kind === "hypothesis_version" && typeof target.hypothesisVersionId === "string")
    || (target.kind === "finding_version" && typeof target.findingVersionId === "string")
    || (target.kind === "assumption" && typeof target.assumptionKey === "string");
  return validTarget
    && ["added", "changed", "removed", "affected"].includes(String(value.impactKind))
    && isNullableString(value.previousStatement)
    && isNullableString(value.currentStatement);
}

function parseJson<T>(
  raw: string,
  label: string,
  isValue: (value: unknown) => value is T,
): T {
  let value: unknown;
  try {
    value = JSON.parse(raw);
  } catch {
    throw new Error(`${label} must contain valid JSON.`);
  }
  if (!isValue(value)) throw new Error(`${label} has an invalid shape.`);
  return value;
}

function parseArray<T>(
  raw: string,
  label: string,
  isItem: (value: unknown) => value is T,
): T[] {
  return parseJson(raw, label, (value): value is T[] =>
    Array.isArray(value) && value.every(isItem));
}

function isJsonObject(value: unknown): value is Record<string, unknown> {
  return isRecord(value);
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function changeClass(value: string): CreateModelVersionReq["changeClass"] {
  switch (value) {
    case "refinement": return "refinement";
    case "extension": return "extension";
    case "revision": return "revision";
    case "replacement": return "replacement";
    default: throw new Error("Choose a supported model change class.");
  }
}

function evidencePolarity(value: string): ModelEvidencePolarity {
  switch (value) {
    case "supports": return "supports";
    case "contradicts": return "contradicts";
    case "qualifies": return "qualifies";
    case "neutral": return "neutral";
    default: throw new Error("Choose a supported evidence polarity.");
  }
}

function artifactReference(kind: string, id: string): ModelArtifactRef {
  switch (kind) {
    case "analysis_output": return { kind, id };
    case "experiment_run": return { kind, id };
    case "dataset_version": return { kind, id };
    case "research_observation": return { kind, id };
    case "source_version_segment": return { kind, id };
    case "source_claim": return { kind, id };
    case "finding_version": return { kind, id };
    default: throw new Error("Choose a supported evidence artifact type.");
  }
}

function validationIndependence(value: string): "independent" | "not_independent" | "unknown" {
  switch (value) {
    case "independent": return "independent";
    case "not_independent": return "not_independent";
    case "unknown": return "unknown";
    default: throw new Error("Choose a supported independence state.");
  }
}

function validationOutcome(value: string): "pass" | "fail" | "inconclusive" {
  switch (value) {
    case "pass": return "pass";
    case "fail": return "fail";
    case "inconclusive": return "inconclusive";
    default: throw new Error("Choose a supported validation outcome.");
  }
}

function replicationKind(value: string): "direct" | "conceptual" {
  switch (value) {
    case "direct": return "direct";
    case "conceptual": return "conceptual";
    default: throw new Error("Choose a supported replication kind.");
  }
}

function replicationOutcome(value: string): "supports" | "contradicts" | "mixed" | "inconclusive" {
  switch (value) {
    case "supports": return "supports";
    case "contradicts": return "contradicts";
    case "mixed": return "mixed";
    case "inconclusive": return "inconclusive";
    default: throw new Error("Choose a supported replication outcome.");
  }
}

function ModelFormLabel({ title, children }: { title: string; children: React.ReactNode }): React.JSX.Element {
  return <label className="block space-y-1 text-xs text-gray-400"><span>{title}</span>{children}</label>;
}

export function ModelWorkspacePanel({ projectId }: { projectId: string }): React.JSX.Element {
  const [workspace, setWorkspace] = useState<ModelWorkspaceDto | null>(null);
  const [analysis, setAnalysis] = useState<AnalysisWorkspaceDto | null>(null);
  const [experiments, setExperiments] = useState<ExperimentWorkspaceDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [selectedModelId, setSelectedModelId] = useState("");
  const [compareLeftId, setCompareLeftId] = useState("");
  const [compareRightId, setCompareRightId] = useState("");
  const [evaluationRunId, setEvaluationRunId] = useState("");

  const refresh = useCallback(async (): Promise<void> => {
    if (!projectId) {
      setWorkspace(null);
      setAnalysis(null);
      setExperiments(null);
      return;
    }
    setBusy(true);
    try {
      const [models, analyses, experimentData] = await Promise.all([
        modelService.getWorkspace(projectId),
        analysisService.getWorkspace(projectId),
        experimentService.getWorkspace(projectId),
      ]);
      setWorkspace(models);
      setAnalysis(analyses);
      setExperiments(experimentData);
      setError(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }, [projectId]);

  useEffect(() => { void refresh(); }, [refresh]);

  const perform = async (operation: () => Promise<unknown>): Promise<void> => {
    setBusy(true);
    try {
      await operation();
      await refresh();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const versionsForModel = useMemo(
    () => (workspace?.versions ?? []).filter((version) => version.modelId === selectedModelId),
    [workspace, selectedModelId],
  );
  const comparisonLeft = workspace?.versions.find((version) => version.id === compareLeftId);
  const comparisonRight = workspace?.versions.find((version) => version.id === compareRightId);
  const selectedAnalysisRun = analysis?.runs.find((run) => run.id === evaluationRunId);

  const createModel = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const form = event.currentTarget;
    const values = new FormData(form);
    await perform(async () => {
      const model = await modelService.create({
        displayName: String(values.get("displayName") ?? ""),
        description: String(values.get("description") ?? ""),
        originProjectId: projectId,
        domainId: null,
        createdBy: null,
      });
      setSelectedModelId(model.id);
      form.reset();
    });
  };

  const createVersion = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const form = event.currentTarget;
    const values = new FormData(form);
    const modelId = String(values.get("modelId") ?? "");
    const assumptionKey = String(values.get("assumptionKey") ?? "").trim();
    const assumptionStatement = String(values.get("assumptionStatement") ?? "").trim();
    await perform(async () => {
      const impacts = parseArray(
        String(values.get("impacts") ?? "[]"),
        "Affected claims/assumptions",
        isImpact,
      );
      if (assumptionKey && assumptionStatement && !impacts.some(
        (impact) => impact.target.kind === "assumption" && impact.target.assumptionKey === assumptionKey,
      )) {
        impacts.push({
          target: { kind: "assumption", assumptionKey },
          impactKind: "added",
          previousStatement: null,
          currentStatement: assumptionStatement,
          rationale: String(values.get("changeRationale") ?? ""),
        });
      }
      const created = await modelService.createVersion({
        projectId,
        modelId,
        changeClass: changeClass(String(values.get("changeClass") ?? "revision")),
        changeRationale: String(values.get("changeRationale") ?? ""),
        expectedImplications: String(values.get("expectedImplications") ?? ""),
        specification: {
          humanReadable: String(values.get("humanSpecification") ?? ""),
          machineReadable: parseJson(
            String(values.get("machineSpecification") ?? "{}"),
            "Machine specification",
            isJsonObject,
          ),
        },
        assumptions: assumptionKey && assumptionStatement
          ? [{ key: assumptionKey, statement: assumptionStatement }]
          : [],
        scope: String(values.get("scope") ?? ""),
        dependencies: parseArray(String(values.get("dependencies") ?? "[]"), "Dependencies", isDependency),
        environment: parseJson(String(values.get("environment") ?? "{}"), "Environment", isJsonObject),
        parents: values.getAll("parentVersionIds").map((parentId) => ({
          versionId: String(parentId),
          rationale: String(values.get("parentRationale") ?? ""),
        })),
        parameters: parseArray(String(values.get("parameters") ?? "[]"), "Parameters", isModelParameter),
        artifacts: parseArray(String(values.get("artifacts") ?? "[]"), "Artifacts", isArtifact),
        impacts,
        hypotheses: parseArray(String(values.get("hypotheses") ?? "[]"), "Hypothesis links", isRelation),
        findings: parseArray(String(values.get("findings") ?? "[]"), "Finding links", isRelation),
        experiments: parseArray(String(values.get("experiments") ?? "[]"), "Experiment links", isExperimentRelation),
        datasetUses: parseArray(String(values.get("datasetUses") ?? "[]"), "Dataset uses", isDatasetUse),
        evidenceIds: parseArray(String(values.get("evidenceIds") ?? "[]"), "Evidence IDs", isString),
        createdBy: null,
      });
      setSelectedModelId(modelId);
      setCompareRightId(created.id);
      form.reset();
    });
  };

  const createEvidence = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const values = new FormData(event.currentTarget);
    const kind = String(values.get("artifactKind") ?? "");
    const artifactId = String(values.get("artifactId") ?? "").trim();
    if (!artifactId) {
      setError("Provide the exact evidence artifact ID.");
      return;
    }
    let artifact: ModelArtifactRef;
    try {
      artifact = artifactReference(kind, artifactId);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
      return;
    }
    await perform(() => modelService.createEvidence({
      modelVersionId: String(values.get("modelVersionId") ?? ""),
      artifact,
      polarity: evidencePolarity(String(values.get("polarity") ?? "supports")),
      scope: String(values.get("scope") ?? ""),
      qualityRationale: String(values.get("qualityRationale") ?? ""),
      limitations: String(values.get("limitations") ?? ""),
      provenance: { recordedIn: "model-workspace" },
      createdBy: null,
    }));
  };

  const createEvaluation = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const values = new FormData(event.currentTarget);
    const run = analysis?.runs.find((item) => item.id === String(values.get("analysisRunId") ?? ""));
    const input = run?.inputs.find((item) => item.id === String(values.get("inputId") ?? ""));
    if (!run || !input || run.lifecycleStatus !== "completed") {
      setError("Select a completed AnalysisRun and one of its exact inputs.");
      return;
    }
    await perform(() => modelService.createEvaluation({
      projectId,
      modelVersionId: String(values.get("modelVersionId") ?? ""),
      analysisRunId: run.id,
      analysisRunInputId: input.id,
      datasetVersionId: input.datasetVersionId,
      analysisMetricId: String(values.get("metricId") ?? ""),
      role: input.role,
      splitGroupId: input.splitGroupId,
      limitations: String(values.get("limitations") ?? ""),
      createdBy: null,
    }));
  };

  const createValidation = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const values = new FormData(event.currentTarget);
    const modelVersionId = String(values.get("modelVersionId") ?? "");
    await perform(() => modelService.createValidation({
      projectId,
      target: { kind: "model_version", id: modelVersionId },
      criteria: [String(values.get("criterion") ?? "")],
      scope: String(values.get("scope") ?? ""),
      populationScope: String(values.get("populationScope") ?? ""),
      evaluationMethod: String(values.get("evaluationMethod") ?? ""),
      evaluatorIdentity: String(values.get("evaluatorIdentity") ?? ""),
      independenceState: validationIndependence(String(values.get("independenceState") ?? "unknown")),
      independenceRationale: String(values.get("independenceRationale") ?? ""),
      conflictOfInterest: String(values.get("conflictOfInterest") ?? ""),
      createdBy: null,
    }));
  };

  const finalizeValidation = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const values = new FormData(event.currentTarget);
    const modelEvidenceIds = values.getAll("modelEvidenceIds").map(String);
    if (modelEvidenceIds.length === 0) {
      setError("Select model evidence before recording a validation outcome.");
      return;
    }
    await perform(() => modelService.finalizeValidation({
      validationId: String(values.get("validationId") ?? ""),
      outcome: validationOutcome(String(values.get("outcome") ?? "inconclusive")),
      rationale: String(values.get("rationale") ?? ""),
      deviations: [String(values.get("deviations") ?? "")].filter((value) => value.trim().length > 0),
      analysisEvidenceIds: [],
      modelEvidenceIds,
      actorId: null,
    }));
  };

  const createReplication = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const values = new FormData(event.currentTarget);
    const runId = String(values.get("originalRunId") ?? "");
    const run = experiments?.runs.find((item) => item.id === runId);
    const evidenceIds = values.getAll("modelEvidenceIds").map(String);
    if (!run || evidenceIds.length === 0) {
      setError("Select an original experiment run and at least one exact model evidence record.");
      return;
    }
    await perform(() => modelService.createReplication({
      projectId,
      modelVersionId: String(values.get("modelVersionId") ?? ""),
      originalExperimentRunId: run.id,
      replicationKind: replicationKind(String(values.get("replicationKind") ?? "direct")),
      independentTeamOperator: String(values.get("operator") ?? ""),
      protocolContext: String(values.get("protocolContext") ?? ""),
      datasetContext: { originalDesignVersionId: run.designVersionId },
      datasets: [],
      outcome: replicationOutcome(String(values.get("outcome") ?? "inconclusive")),
      resultSummary: String(values.get("resultSummary") ?? ""),
      deviations: [String(values.get("deviations") ?? "")].filter((value) => value.trim().length > 0),
      provenance: { recordedIn: "model-workspace" },
      analysisEvidenceIds: [],
      modelEvidenceIds: evidenceIds,
      createdBy: null,
    }));
  };

  const projectRuns = (analysis?.runs ?? []).filter((run) => run.lifecycleStatus === "completed");
  const modelEvidence = workspace?.evidence ?? [];
  const modelVersions = workspace?.versions ?? [];
  if (!projectId) return <></>;

  return (
    <section className="space-y-4 rounded-xl border border-[#292c34] bg-[#17191f] p-4">
      <header className="flex items-start justify-between gap-4">
        <div>
          <h3 className="font-semibold text-white">Model development and scientific validation</h3>
          <p className="mt-1 text-xs text-gray-400">
            Model identities are independent of hypotheses. Versions and outcomes are immutable records; validation scope is not a claim of scientific truth.
          </p>
        </div>
        <button type="button" className={buttonClass} onClick={() => void refresh()} disabled={busy} aria-label="Refresh model workspace">
          <RefreshCw size={14} className={busy ? "animate-spin" : ""} />
        </button>
      </header>
      {error && <div role="alert" className="flex gap-2 rounded-lg border border-rose-900/60 bg-rose-950/30 px-3 py-2 text-xs text-rose-200"><AlertCircle size={14} />{error}</div>}

      <div className="grid gap-3 sm:grid-cols-4">
        <p className="rounded-lg border border-[#343741] p-3 text-xs text-gray-300"><b className="text-lg text-white">{workspace?.models.length ?? 0}</b><br />Reusable model identities</p>
        <p className="rounded-lg border border-[#343741] p-3 text-xs text-gray-300"><b className="text-lg text-white">{workspace?.versions.length ?? 0}</b><br />Immutable versions</p>
        <p className="rounded-lg border border-[#343741] p-3 text-xs text-gray-300"><b className="text-lg text-white">{workspace?.evaluations.length ?? 0}</b><br />Exact-version evaluations</p>
        <p className="rounded-lg border border-[#343741] p-3 text-xs text-gray-300"><b className="text-lg text-white">{workspace?.replications.length ?? 0}</b><br />Independent replications</p>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <form onSubmit={(event) => void createModel(event)} className="space-y-3 rounded-lg border border-[#343741] p-3">
          <h4 className="text-sm font-medium text-gray-200">Register a reusable model</h4>
          <ModelFormLabel title="Model name"><input className={fieldClass} name="displayName" required /></ModelFormLabel>
          <ModelFormLabel title="Purpose and description"><textarea className={fieldClass} name="description" rows={2} /></ModelFormLabel>
          <button className={buttonClass} disabled={busy}>Create stable identity</button>
        </form>

        <form onSubmit={(event) => void createVersion(event)} className="space-y-3 rounded-lg border border-[#343741] p-3">
          <h4 className="text-sm font-medium text-gray-200">Create immutable version</h4>
          <ModelFormLabel title="Model">
            <select className={fieldClass} name="modelId" value={selectedModelId} onChange={(event) => setSelectedModelId(event.target.value)} required>
              <option value="">Select a model</option>
              {(workspace?.models ?? []).map((model) => <option key={model.id} value={model.id}>{model.displayName} · {model.status}</option>)}
            </select>
          </ModelFormLabel>
          <div className="grid grid-cols-2 gap-2">
            <ModelFormLabel title="Change class">
              <select className={fieldClass} name="changeClass" defaultValue="revision">
                <option value="refinement">Refinement</option><option value="extension">Extension</option>
                <option value="revision">Revision / initial baseline</option><option value="replacement">Replacement</option>
              </select>
            </ModelFormLabel>
            <ModelFormLabel title="Parent versions (successors)">
              <select className={fieldClass} name="parentVersionIds" multiple size={2}>
                {versionsForModel.map((version) => <option key={version.id} value={version.id}>v{version.versionNo} · {version.id}</option>)}
              </select>
            </ModelFormLabel>
          </div>
          <ModelFormLabel title="Change rationale"><textarea className={fieldClass} name="changeRationale" required rows={2} /></ModelFormLabel>
          <ModelFormLabel title="Expected implications"><textarea className={fieldClass} name="expectedImplications" required rows={2} /></ModelFormLabel>
          <ModelFormLabel title="Human-readable specification"><textarea className={fieldClass} name="humanSpecification" required rows={3} /></ModelFormLabel>
          <ModelFormLabel title="Machine-readable specification (JSON object)">
            <textarea className={fieldClass} name="machineSpecification" defaultValue="{}" required rows={2} />
          </ModelFormLabel>
          <div className="grid grid-cols-2 gap-2">
            <ModelFormLabel title="Scope"><input className={fieldClass} name="scope" required /></ModelFormLabel>
            <ModelFormLabel title="Parent rationale"><input className={fieldClass} name="parentRationale" defaultValue="Version lineage." /></ModelFormLabel>
          </div>
          <div className="grid grid-cols-2 gap-2">
            <ModelFormLabel title="Assumption key"><input className={fieldClass} name="assumptionKey" /></ModelFormLabel>
            <ModelFormLabel title="Assumption statement"><input className={fieldClass} name="assumptionStatement" /></ModelFormLabel>
          </div>
          <details className="rounded border border-[#343741] p-2">
            <summary className="cursor-pointer text-xs text-gray-300">Parameters, artifacts, and typed contribution links (JSON arrays)</summary>
            <div className="mt-3 grid gap-2 sm:grid-cols-2">
              <ModelFormLabel title="Parameters"><textarea className={fieldClass} name="parameters" defaultValue="[]" rows={3} /></ModelFormLabel>
              <ModelFormLabel title="Dependencies"><textarea className={fieldClass} name="dependencies" defaultValue="[]" rows={3} /></ModelFormLabel>
              <ModelFormLabel title="Artifacts"><textarea className={fieldClass} name="artifacts" defaultValue="[]" rows={3} /></ModelFormLabel>
              <ModelFormLabel title="Affected claims/assumptions"><textarea className={fieldClass} name="impacts" defaultValue="[]" rows={3} /></ModelFormLabel>
              <ModelFormLabel title="Hypothesis links"><textarea className={fieldClass} name="hypotheses" defaultValue="[]" rows={3} /></ModelFormLabel>
              <ModelFormLabel title="Finding links"><textarea className={fieldClass} name="findings" defaultValue="[]" rows={3} /></ModelFormLabel>
              <ModelFormLabel title="Experiment links"><textarea className={fieldClass} name="experiments" defaultValue="[]" rows={3} /></ModelFormLabel>
              <ModelFormLabel title="Dataset uses"><textarea className={fieldClass} name="datasetUses" defaultValue="[]" rows={3} /></ModelFormLabel>
              <ModelFormLabel title="Existing model evidence IDs"><textarea className={fieldClass} name="evidenceIds" defaultValue="[]" rows={2} /></ModelFormLabel>
              <ModelFormLabel title="Environment identity (JSON object)"><textarea className={fieldClass} name="environment" defaultValue="{}" rows={2} /></ModelFormLabel>
            </div>
          </details>
          <button className={buttonClass} disabled={busy || !selectedModelId}>Save new version</button>
        </form>
      </div>

      <div className="space-y-2">
        <h4 className="text-sm font-medium text-gray-200">Registry, specification, contributions, and contradictions</h4>
        {(workspace?.models ?? []).map((model) => (
          <details key={model.id} className="rounded-lg border border-[#343741] p-3" open={model.id === selectedModelId}>
            <summary className="cursor-pointer text-sm font-medium text-white">{model.displayName} · {model.status} · {model.id}</summary>
            <p className="mt-2 text-xs text-gray-400">{model.description} · origin {model.originProjectId ?? "workspace reusable"}</p>
            {(workspace?.versions ?? []).filter((version) => version.modelId === model.id).map((version) => (
              <details key={version.id} className="mt-2 rounded border border-[#343741] p-3">
                <summary className="cursor-pointer text-xs text-gray-200">
                  v{version.versionNo} · {version.changeClass} · {version.createdAtUs} · SHA-256 {version.contentSha256}
                </summary>
                <div className="mt-2 space-y-2 text-xs text-gray-400">
                  <p>{version.changeRationale} · implications: {version.expectedImplications}</p>
                  <p>Parents: {version.parents.map((parent) => parent.versionId).join(", ") || "initial baseline"}</p>
                  <p>Scope: {version.scope}</p>
                  <pre className="overflow-auto whitespace-pre-wrap rounded bg-[#101114] p-2">{version.specification.humanReadable}{"\n"}{JSON.stringify(version.specification.machineReadable, null, 2)}</pre>
                  <p>Parameters: {JSON.stringify(version.parameters)} · assumptions: {JSON.stringify(version.assumptions)}</p>
                  <p>Dependencies: {JSON.stringify(version.dependencies)} · artifacts: {JSON.stringify(version.artifacts)}</p>
                  <p>Impacts: {JSON.stringify(version.impacts)}</p>
                  <p>Hypotheses: {JSON.stringify(version.hypotheses)} · findings: {JSON.stringify(version.findings)}</p>
                  <p>Experiments: {JSON.stringify(version.experiments)} · exact dataset uses: {JSON.stringify(version.datasetUses)}</p>
                  <p>Evidence IDs: {version.evidenceIds.join(", ") || "none"}</p>
                </div>
              </details>
            ))}
          </details>
        ))}
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <div className="space-y-3 rounded-lg border border-[#343741] p-3">
          <h4 className="text-sm font-medium text-gray-200">Version comparison</h4>
          <div className="grid grid-cols-2 gap-2">
            <select className={fieldClass} value={compareLeftId} onChange={(event) => setCompareLeftId(event.target.value)}>
              <option value="">Earlier version</option>
              {modelVersions.map((version) => <option key={version.id} value={version.id}>{version.modelId} · v{version.versionNo}</option>)}
            </select>
            <select className={fieldClass} value={compareRightId} onChange={(event) => setCompareRightId(event.target.value)}>
              <option value="">Later version</option>
              {modelVersions.map((version) => <option key={version.id} value={version.id}>{version.modelId} · v{version.versionNo}</option>)}
            </select>
          </div>
          {comparisonLeft && comparisonRight && (
            <div className="grid gap-2 md:grid-cols-2">
              {[comparisonLeft, comparisonRight].map((version) => (
                <pre key={version.id} className="overflow-auto whitespace-pre-wrap rounded bg-[#101114] p-2 text-[11px] text-gray-300">
                  {JSON.stringify({ version: version.versionNo, changeClass: version.changeClass, rationale: version.changeRationale, specification: version.specification, parameters: version.parameters, assumptions: version.assumptions, artifacts: version.artifacts }, null, 2)}
                </pre>
              ))}
            </div>
          )}
        </div>

        <form onSubmit={(event) => void createEvidence(event)} className="space-y-2 rounded-lg border border-[#343741] p-3">
          <h4 className="text-sm font-medium text-gray-200">Record typed model evidence</h4>
          <select className={fieldClass} name="modelVersionId" required defaultValue="">
            <option value="">Target model version</option>
            {modelVersions.map((version) => <option key={version.id} value={version.id}>{version.modelId} · v{version.versionNo}</option>)}
          </select>
          <div className="grid grid-cols-2 gap-2">
            <select className={fieldClass} name="artifactKind" defaultValue="analysis_output">
              <option value="analysis_output">Analysis output</option><option value="experiment_run">Experiment run</option>
              <option value="dataset_version">Dataset version</option><option value="research_observation">Observation</option>
              <option value="source_version_segment">Source segment</option><option value="source_claim">Source claim</option>
              <option value="finding_version">Finding version</option>
            </select>
            <input className={fieldClass} name="artifactId" placeholder="Exact artifact ID" required />
          </div>
          <div className="grid grid-cols-2 gap-2">
            <select className={fieldClass} name="polarity"><option value="supports">Supports</option><option value="contradicts">Contradicts</option><option value="qualifies">Qualifies</option><option value="neutral">Neutral</option></select>
            <input className={fieldClass} name="scope" placeholder="Scope" required />
          </div>
          <textarea className={fieldClass} name="qualityRationale" placeholder="Methodological quality rationale" required />
          <textarea className={fieldClass} name="limitations" placeholder="Limitations" required />
          <button className={buttonClass} disabled={busy}>Record evidence</button>
        </form>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <form onSubmit={(event) => void createEvaluation(event)} className="space-y-2 rounded-lg border border-[#343741] p-3">
          <h4 className="text-sm font-medium text-gray-200">Evaluate an exact model version</h4>
          <select className={fieldClass} name="modelVersionId" required defaultValue="">
            <option value="">Model version</option>
            {modelVersions.map((version) => <option key={version.id} value={version.id}>{version.modelId} · v{version.versionNo}</option>)}
          </select>
          <select className={fieldClass} name="analysisRunId" required value={evaluationRunId} onChange={(event) => setEvaluationRunId(event.target.value)}>
            <option value="">Completed AnalysisRun</option>
            {projectRuns.map((run) => <option key={run.id} value={run.id}>{run.id}</option>)}
          </select>
          <p className="text-[11px] text-gray-500">The backend checks that the selected input and metric belong to the same completed run.</p>
          <select className={fieldClass} name="inputId" required defaultValue="">
            <option value="">Exact run input</option>
            {(selectedAnalysisRun?.inputs ?? []).map((input) => (
              <option key={input.id} value={input.id}>{input.role} · {input.datasetVersionId} · {input.id}</option>
            ))}
          </select>
          <select className={fieldClass} name="metricId" required defaultValue="">
            <option value="">Analysis metric</option>
            (selectedAnalysisRun?.metrics ?? []).map((metric) => (
              <option key={metric.id} value={metric.id}>{metric.estimand} · {metric.effectEstimate} {metric.unit} · {metric.id}</option>
            ))
          </select>
          <textarea className={fieldClass} name="limitations" placeholder="Evaluation scope and limitations" required />
          <button className={buttonClass} disabled={busy || projectRuns.length === 0}>Save evaluation</button>
          {(workspace?.evaluations ?? []).map((evaluation) => (
            <p key={evaluation.id} className="break-all text-xs text-gray-400">
              {evaluation.modelVersionId} · {evaluation.effectEstimate} {evaluation.unit} ({evaluation.estimand}, n={evaluation.sampleSize}) · {evaluation.role} · dataset {evaluation.datasetVersionId} · run {evaluation.analysisRunId}
            </p>
          ))}
        </form>

        <form onSubmit={(event) => void createValidation(event)} className="space-y-2 rounded-lg border border-[#343741] p-3">
          <h4 className="text-sm font-medium text-gray-200">Predeclare a scoped validation</h4>
          <select className={fieldClass} name="modelVersionId" required defaultValue="">
            <option value="">Target ModelVersion</option>
            {modelVersions.map((version) => <option key={version.id} value={version.id}>{version.modelId} · v{version.versionNo}</option>)}
          </select>
          <input className={fieldClass} name="criterion" placeholder="Predeclared criterion" required />
          <input className={fieldClass} name="scope" placeholder="Criterion scope" required />
          <input className={fieldClass} name="populationScope" placeholder="Data/population scope" required />
          <input className={fieldClass} name="evaluationMethod" placeholder="Evaluation method" required />
          <input className={fieldClass} name="evaluatorIdentity" placeholder="Evaluator identity" required />
          <select className={fieldClass} name="independenceState"><option value="independent">Independent</option><option value="not_independent">Not independent</option><option value="unknown">Unknown</option></select>
          <input className={fieldClass} name="independenceRationale" placeholder="Independence rationale (required unless independent)" />
          <input className={fieldClass} name="conflictOfInterest" placeholder="Conflict declaration (explicitly state none if applicable)" required />
          <button className={buttonClass} disabled={busy}>Create planned validation</button>
        </form>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <form onSubmit={(event) => void finalizeValidation(event)} className="space-y-2 rounded-lg border border-[#343741] p-3">
          <h4 className="text-sm font-medium text-gray-200">Record validation result</h4>
          <select className={fieldClass} name="validationId" required defaultValue="">
            <option value="">Planned validation</option>
            {(workspace?.validations ?? []).filter((validation) => validation.outcome === null).map((validation) => (
              <option key={validation.id} value={validation.id}>{validation.evaluatorIdentity} · {validation.scope}</option>
            ))}
          </select>
          <select className={fieldClass} name="outcome"><option value="pass">Pass</option><option value="fail">Fail</option><option value="inconclusive">Inconclusive</option></select>
          <textarea className={fieldClass} name="rationale" placeholder="Outcome rationale" required />
          <textarea className={fieldClass} name="deviations" placeholder="Deviations (if any)" />
          <select className={fieldClass} name="modelEvidenceIds" multiple size={3} required>
            {modelEvidence.map((item) => <option key={item.id} value={item.id}>{item.polarity} · {item.id}</option>)}
          </select>
          <button className={buttonClass} disabled={busy || modelEvidence.length === 0}>Finalize once</button>
        </form>

        <form onSubmit={(event) => void createReplication(event)} className="space-y-2 rounded-lg border border-[#343741] p-3">
          <h4 className="text-sm font-medium text-gray-200">Record independent replication</h4>
          <select className={fieldClass} name="modelVersionId" required defaultValue="">
            <option value="">Target ModelVersion</option>
            {modelVersions.map((version) => <option key={version.id} value={version.id}>{version.modelId} · v{version.versionNo}</option>)}
          </select>
          <select className={fieldClass} name="originalRunId" required defaultValue="">
            <option value="">Original completed experiment run</option>
            {(experiments?.runs ?? []).filter((run) => run.lifecycleStatus === "completed" || run.lifecycleStatus === "inconclusive").map((run) => (
              <option key={run.id} value={run.id}>{run.id} · design {run.designVersionId}</option>
            ))}
          </select>
          <select className={fieldClass} name="replicationKind"><option value="direct">Direct</option><option value="conceptual">Conceptual</option></select>
          <input className={fieldClass} name="operator" placeholder="Independent team/operator" required />
          <input className={fieldClass} name="protocolContext" placeholder="Protocol context" required />
          <select className={fieldClass} name="outcome"><option value="supports">Supports original</option><option value="contradicts">Contradicts original</option><option value="mixed">Mixed</option><option value="inconclusive">Inconclusive</option></select>
          <textarea className={fieldClass} name="resultSummary" placeholder="Reported result" required />
          <textarea className={fieldClass} name="deviations" placeholder="Deviations (if any)" />
          <select className={fieldClass} name="modelEvidenceIds" multiple size={3} required>
            {modelEvidence.map((item) => <option key={item.id} value={item.id}>{item.polarity} · {item.id}</option>)}
          </select>
          <button className={buttonClass} disabled={busy || modelEvidence.length === 0}>Preserve replication result</button>
        </form>
      </div>

      <div className="space-y-2">
        <h4 className="text-sm font-medium text-gray-200">Validation and replication history</h4>
        {(workspace?.validations ?? []).map((validation) => (
          <p key={validation.id} className="rounded border border-[#343741] p-2 text-xs text-gray-300">
            Validation {validation.outcome ?? "planned"} · {validation.target.kind}:{validation.target.id} · criterion {JSON.stringify(validation.criteria)} · {validation.evaluatorIdentity} ({validation.independenceState}; COI: {validation.conflictOfInterest})
          </p>
        ))}
        {(workspace?.replications ?? []).map((replication) => (
          <p key={replication.id} className="rounded border border-[#343741] p-2 text-xs text-gray-300">
            {replication.replicationKind} replication · {replication.outcome} · model {replication.modelVersionId} · original run {replication.originalExperimentRunId} / design {replication.originalDesignVersionId} · {replication.independentTeamOperator}: {replication.resultSummary}
          </p>
        ))}
      </div>
    </section>
  );
}
