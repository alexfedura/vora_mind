import React, { useCallback, useEffect, useMemo, useState } from "react";
import { AlertCircle, RefreshCw } from "lucide-react";
import { analysisService } from "@/mind/services/analysisService";
import type {
  AnalysisRunDto,
  AnalysisWorkspaceDto,
  EvidenceArtifactRefDto,
  EvidencePolarity,
  EvidenceStatus,
  FindingConclusionKind,
  FindingStatus,
  PropositionRefDto,
} from "@/types";

const inputClass =
  "w-full rounded-lg border border-[#343741] bg-[#101114] px-3 py-2 text-sm text-white outline-none focus:border-[#e8ad55]";
const buttonClass =
  "rounded-lg border border-[#343741] px-3 py-2 text-xs font-medium text-gray-200 hover:bg-[#202229] disabled:opacity-40";

function isPropositionKind(value: string): value is PropositionRefDto["kind"] {
  return value === "source_claim" || value === "hypothesis_version" || value === "finding_version";
}

function isArtifactKind(value: string): value is EvidenceArtifactRefDto["kind"] {
  return value === "analysis_output"
    || value === "experiment_run"
    || value === "dataset_version"
    || value === "research_observation"
    || value === "source_version_segment"
    || value === "source_claim";
}

function nextEvidenceStatuses(status: EvidenceStatus): EvidenceStatus[] {
  switch (status) {
    case "recorded":
      return ["assessed", "contested", "rejected", "retracted"];
    case "assessed":
      return ["accepted", "contested", "rejected", "retracted"];
    case "accepted":
      return ["contested", "superseded", "retracted"];
    case "contested":
      return ["accepted", "rejected", "superseded", "retracted"];
    default:
      return [];
  }
}

function nextFindingStatuses(status: FindingStatus): FindingStatus[] {
  switch (status) {
    case "draft":
      return ["review", "retracted"];
    case "review":
      return ["accepted", "contested", "unsupported", "inconclusive", "retracted"];
    case "accepted":
      return ["contested", "superseded", "retracted"];
    case "contested":
      return ["accepted", "unsupported", "inconclusive", "superseded", "retracted"];
    case "unsupported":
    case "inconclusive":
      return ["contested", "superseded", "retracted"];
    default:
      return [];
  }
}

function evidencePolarity(value: string): EvidencePolarity {
  switch (value) {
    case "supports":
    case "contradicts":
    case "qualifies":
    case "neutral":
      return value;
    default:
      throw new Error(`Unsupported evidence polarity: ${value}`);
  }
}

function conclusionKind(value: string): FindingConclusionKind {
  switch (value) {
    case "result":
    case "null_result":
    case "contradictory":
    case "inconclusive":
      return value;
    default:
      throw new Error(`Unsupported finding conclusion: ${value}`);
  }
}

function isFindingProposition(
  proposition: PropositionRefDto,
  kind: PropositionRefDto["kind"],
  id: string,
): boolean {
  return proposition.kind === kind && proposition.id === id;
}

function formatTime(timestampUs: number): string {
  return new Date(timestampUs / 1000).toLocaleString();
}

function RunCard({ run }: { run: AnalysisRunDto }): React.JSX.Element {
  return (
    <details className="rounded-lg border border-[#343741] bg-[#101114] p-3">
      <summary className="cursor-pointer list-none">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="font-medium text-white">{run.id}</span>
          <span className="rounded-md bg-[#252832] px-2 py-1 text-[11px] text-gray-200">
            {run.lifecycleStatus}
          </span>
        </div>
        <p className="mt-1 text-xs text-gray-400">
          Definition {run.definitionVersionId} · protocol {run.protocolVersion} · created {formatTime(run.createdAtUs)}
        </p>
      </summary>
      <div className="mt-4 space-y-4 border-t border-[#292c34] pt-3">
        <div>
          <h5 className="mb-2 text-xs font-semibold uppercase tracking-wide text-gray-400">Pinned inputs</h5>
          {run.inputs.length === 0 ? <p className="text-xs text-gray-500">No input pins recorded.</p> : (
            <div className="space-y-2">
              {run.inputs.map((input) => (
                <details key={input.id} className="rounded-md border border-[#292c34] p-2">
                  <summary className="cursor-pointer text-xs text-gray-200">
                    {input.role} · {input.datasetVersionId} · {input.sampleCount} samples
                  </summary>
                  <div className="mt-2 space-y-1 break-all text-[11px] text-gray-400">
                    <p>SHA-256: {input.datasetChecksum}</p>
                    <p>Split group: {input.splitGroupId ?? "not assigned"}</p>
                    <p>Transformation lineage: {JSON.stringify(input.transformationLineage)}</p>
                    <p>Samples: {input.samples.map((sample) => sample.sampleId).join(", ")}</p>
                  </div>
                </details>
              ))}
            </div>
          )}
        </div>

        <div>
          <h5 className="mb-2 text-xs font-semibold uppercase tracking-wide text-gray-400">Statistical integrity declarations</h5>
          <div className="grid gap-2 sm:grid-cols-2">
            {run.integrityDeclarations.map((declaration) => (
              <div key={declaration.category} className="rounded-md border border-[#292c34] p-2 text-xs">
                <div className="flex items-center justify-between gap-2">
                  <span className="capitalize text-gray-200">{declaration.category.replaceAll("_", " ")}</span>
                  <span className={declaration.applicability === "not_assessed" ? "text-amber-300" : "text-gray-400"}>
                    {declaration.applicability.replaceAll("_", " ")}
                  </span>
                </div>
                {declaration.rationale && <p className="mt-1 text-gray-400">{declaration.rationale}</p>}
                {declaration.applicability === "applicable" && (
                  <pre className="mt-1 overflow-auto whitespace-pre-wrap text-[10px] text-gray-500">
                    {JSON.stringify(declaration.details, null, 2)}
                  </pre>
                )}
              </div>
            ))}
          </div>
          {run.integrityFindings.length > 0 && (
            <ul className="mt-2 space-y-1">
              {run.integrityFindings.map((finding) => (
                <li key={finding.id} className="text-xs text-amber-200">
                  {finding.severity}: {finding.message}
                </li>
              ))}
            </ul>
          )}
        </div>

        <div>
          <h5 className="mb-2 text-xs font-semibold uppercase tracking-wide text-gray-400">Outputs and statistical reports</h5>
          {run.outputs.length === 0 && <p className="text-xs text-gray-500">No outputs recorded.</p>}
          <div className="space-y-2">
            {run.outputs.map((output) => (
              <div key={output.id} className="rounded-md border border-[#292c34] p-2">
                <p className="text-xs font-medium text-gray-200">{output.title} <span className="text-gray-500">({output.outputKind})</span></p>
                <p className="mt-1 break-all text-[11px] text-gray-400">
                  {output.artifactUri ? `${output.artifactUri} · SHA-256 ${output.artifactSha256}` : `Inline output · SHA-256 ${output.payloadSha256}`}
                </p>
                <pre className="mt-2 max-h-36 overflow-auto whitespace-pre-wrap text-[11px] text-gray-400">
                  {JSON.stringify(output.payload, null, 2)}
                </pre>
                {run.metrics.filter((metric) => metric.outputId === output.id).map((metric) => (
                  <div key={metric.id} className="mt-2 border-t border-[#292c34] pt-2 text-xs text-gray-300">
                    <p>{metric.estimand}: {metric.effectEstimate} {metric.unit} (n={metric.sampleSize})</p>
                    <p>Uncertainty: {metric.uncertainty.kind === "interval"
                      ? `${metric.uncertainty.interval.lower}–${metric.uncertainty.interval.upper} (${metric.uncertainty.interval.level})`
                      : metric.uncertainty.reason}</p>
                    <p>Method version: {metric.methodVersionId} · metric version: {metric.metricDefinitionVersionId}</p>
                    <p>Multiplicity: {metric.multiplicityTreatment}{metric.pValue ? ` · p=${metric.pValue}` : ""}</p>
                    <p>Missing data: {metric.missingData.policy}; {metric.missingData.missingCount} / {metric.sampleSize} missing</p>
                    <p>Assumptions: {metric.assumptions.join("; ") || "none recorded"} · Exclusions: {metric.exclusions.join("; ") || "none recorded"}</p>
                  </div>
                ))}
              </div>
            ))}
          </div>
        </div>

        {run.failureSummary && <p className="text-xs text-rose-300">Failure: {run.failureSummary}</p>}
        {run.errors.map((error) => <p key={error.id} className="text-xs text-rose-300">{error.errorKind}: {error.message}</p>)}
        {run.deviations.map((deviation) => (
          <p key={deviation.id} className="text-xs text-amber-200">
            Deviation: {deviation.description} — {deviation.impactAssessment}
          </p>
        ))}
        {run.completionSummary && <p className="text-xs text-gray-300">Summary: {run.completionSummary}</p>}
      </div>
    </details>
  );
}

export function AnalysisWorkspacePanel({ projectId }: { projectId: string }): React.JSX.Element {
  const [workspace, setWorkspace] = useState<AnalysisWorkspaceDto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [findingPropositionKind, setFindingPropositionKind] =
    useState<PropositionRefDto["kind"]>("source_claim");
  const [findingPropositionId, setFindingPropositionId] = useState("");

  const refresh = useCallback(async (): Promise<void> => {
    if (!projectId) {
      setWorkspace(null);
      return;
    }
    setBusy(true);
    try {
      setWorkspace(await analysisService.getWorkspace(projectId));
      setError(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }, [projectId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const perform = async (operation: () => Promise<unknown>): Promise<void> => {
    setBusy(true);
    try {
      await operation();
      await refresh();
      setError(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  };

  const createEvidence = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const form = event.currentTarget;
    const values = new FormData(form);
    const propositionKind = String(values.get("propositionKind") ?? "");
    const artifactKind = String(values.get("artifactKind") ?? "");
    const propositionId = String(values.get("propositionId") ?? "").trim();
    const artifactId = String(values.get("artifactId") ?? "").trim();
    if (!isPropositionKind(propositionKind) || !isArtifactKind(artifactKind) || !propositionId || !artifactId) {
      setError("Choose typed evidence endpoints and provide both endpoint IDs.");
      return;
    }
    await perform(async () => {
      await analysisService.createEvidence({
        projectId,
        proposition: { kind: propositionKind, id: propositionId },
        artifact: { kind: artifactKind, id: artifactId },
        polarity: evidencePolarity(String(values.get("polarity") ?? "")),
        scope: String(values.get("scope") ?? ""),
        qualityRationale: String(values.get("qualityRationale") ?? ""),
        limitations: String(values.get("limitations") ?? ""),
        provenance: { capturedIn: "analysis-workspace" },
        createdBy: null,
      });
      form.reset();
    });
  };

  const createFinding = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    const form = event.currentTarget;
    const values = new FormData(form);
    const propositionKind = String(values.get("propositionKind") ?? "");
    const propositionId = String(values.get("propositionId") ?? "").trim();
    if (!isPropositionKind(propositionKind) || propositionKind === "finding_version" || !propositionId) {
      setError("A finding must target a source claim or hypothesis version.");
      return;
    }
    const runIds = values.getAll("analysisRunIds").map(String);
    const evidenceIds = values.getAll("evidenceIds").map(String);
    if (runIds.length === 0 || evidenceIds.length === 0) {
      setError("Select at least one completed analysis and proposition-matched evidence record.");
      return;
    }
    await perform(async () => {
      await analysisService.createFindingVersion({
        projectId,
        findingId: null,
        parentVersionId: null,
        title: String(values.get("title") ?? ""),
        statement: String(values.get("statement") ?? ""),
        proposition: { kind: propositionKind, id: propositionId },
        scope: String(values.get("scope") ?? ""),
        applicability: String(values.get("applicability") ?? ""),
        uncertainty: String(values.get("uncertainty") ?? ""),
        limitations: String(values.get("limitations") ?? ""),
        conclusionKind: conclusionKind(String(values.get("conclusionKind") ?? "")),
        analysisRunIds: runIds,
        evidence: evidenceIds.map((evidenceId) => ({
          evidenceId,
          synthesisPolarity: evidencePolarity(String(values.get("synthesisPolarity") ?? "")),
          rationale: String(values.get("synthesisRationale") ?? ""),
        })),
        changeRationale: "",
        createdBy: null,
      });
      form.reset();
    });
  };

  const projectRuns = useMemo(
    () => (workspace?.runs ?? []).filter((run) => run.projectId === projectId),
    [workspace, projectId],
  );
  const completedRuns = projectRuns.filter((run) =>
    run.lifecycleStatus === "completed" || run.lifecycleStatus === "inconclusive",
  );

  if (!projectId) return <></>;

  return (
    <section className="space-y-4 rounded-xl border border-[#292c34] bg-[#17191f] p-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h3 className="font-semibold text-white">Analysis, evidence, and findings</h3>
          <p className="mt-1 text-xs text-gray-400">
            External runs retain exact version pins. Unassessed safeguards remain visible; findings are scoped assessments, not verified facts.
          </p>
        </div>
        <button type="button" className={buttonClass} onClick={() => void refresh()} disabled={busy} aria-label="Refresh analysis workspace">
          <RefreshCw size={14} className={busy ? "animate-spin" : ""} />
        </button>
      </div>

      {error && (
        <div role="alert" className="flex items-center gap-2 rounded-lg border border-rose-900/60 bg-rose-950/30 px-3 py-2 text-xs text-rose-200">
          <AlertCircle size={14} /> {error}
        </div>
      )}

      <div className="grid gap-3 sm:grid-cols-3">
        <div className="rounded-lg border border-[#343741] p-3 text-xs text-gray-300">
          <p className="text-lg font-semibold text-white">{workspace?.definitions.length ?? 0}</p>Immutable analysis definition versions
        </div>
        <div className="rounded-lg border border-[#343741] p-3 text-xs text-gray-300">
          <p className="text-lg font-semibold text-white">{projectRuns.length}</p>Independent analysis runs
        </div>
        <div className="rounded-lg border border-[#343741] p-3 text-xs text-gray-300">
          <p className="text-lg font-semibold text-white">{workspace?.findings.length ?? 0}</p>Versioned findings
        </div>
      </div>

      <div className="space-y-2">
        <h4 className="text-sm font-medium text-gray-200">Analysis run inspection</h4>
        {projectRuns.length === 0
          ? <p className="rounded-lg border border-dashed border-[#343741] p-4 text-xs text-gray-500">No analysis runs are recorded for this project.</p>
          : projectRuns.map((run) => <RunCard key={run.id} run={run} />)}
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <form onSubmit={(event) => void createEvidence(event)} className="space-y-3 rounded-lg border border-[#343741] p-3">
          <h4 className="text-sm font-medium text-gray-200">Record typed evidence</h4>
          <div className="grid grid-cols-2 gap-2">
            <label className="text-xs text-gray-400">Proposition type
              <select className={inputClass} name="propositionKind" required>
                <option value="source_claim">Source claim</option>
                <option value="hypothesis_version">Hypothesis version</option>
                <option value="finding_version">Finding version</option>
              </select>
            </label>
            <label className="text-xs text-gray-400">Proposition ID<input className={inputClass} name="propositionId" required /></label>
          </div>
          <div className="grid grid-cols-2 gap-2">
            <label className="text-xs text-gray-400">Artifact type
              <select className={inputClass} name="artifactKind" required>
                <option value="analysis_output">Analysis output</option>
                <option value="experiment_run">Experiment run</option>
                <option value="dataset_version">Dataset version</option>
                <option value="research_observation">Research observation</option>
                <option value="source_version_segment">Source segment</option>
                <option value="source_claim">Source claim</option>
              </select>
            </label>
            <label className="text-xs text-gray-400">Artifact ID<input className={inputClass} name="artifactId" required /></label>
          </div>
          <div className="grid grid-cols-2 gap-2">
            <label className="text-xs text-gray-400">Polarity
              <select className={inputClass} name="polarity">
                <option value="supports">Supports</option><option value="contradicts">Contradicts</option>
                <option value="qualifies">Qualifies</option><option value="neutral">Neutral</option>
              </select>
            </label>
            <label className="text-xs text-gray-400">Scope<input className={inputClass} name="scope" required /></label>
          </div>
          <label className="block text-xs text-gray-400">Methodological quality rationale<textarea className={inputClass} name="qualityRationale" required rows={2} /></label>
          <label className="block text-xs text-gray-400">Limitations<textarea className={inputClass} name="limitations" required rows={2} /></label>
          <button className={buttonClass} disabled={busy}>Record evidence</button>
        </form>

        <form onSubmit={(event) => void createFinding(event)} className="space-y-3 rounded-lg border border-[#343741] p-3">
          <h4 className="text-sm font-medium text-gray-200">Synthesize a finding version</h4>
          <div className="grid grid-cols-2 gap-2">
            <label className="text-xs text-gray-400">Proposition type
              <select
                className={inputClass}
                name="propositionKind"
                value={findingPropositionKind}
                onChange={(event) => {
                  if (isPropositionKind(event.target.value) && event.target.value !== "finding_version") {
                    setFindingPropositionKind(event.target.value);
                  }
                }}
              >
                <option value="source_claim">Source claim</option>
                <option value="hypothesis_version">Hypothesis version</option>
              </select>
            </label>
            <label className="text-xs text-gray-400">Proposition ID
              <input
                className={inputClass}
                name="propositionId"
                value={findingPropositionId}
                onChange={(event) => setFindingPropositionId(event.target.value)}
                required
              />
            </label>
          </div>
          <label className="block text-xs text-gray-400">Title<input className={inputClass} name="title" required /></label>
          <label className="block text-xs text-gray-400">Finding statement<textarea className={inputClass} name="statement" required rows={2} /></label>
          <div className="grid grid-cols-2 gap-2">
            <label className="text-xs text-gray-400">Conclusion
              <select className={inputClass} name="conclusionKind">
                <option value="result">Result</option><option value="null_result">Null result</option>
                <option value="contradictory">Contradictory</option><option value="inconclusive">Inconclusive</option>
              </select>
            </label>
            <label className="text-xs text-gray-400">Scope<input className={inputClass} name="scope" required /></label>
          </div>
          <label className="block text-xs text-gray-400">Applicability<input className={inputClass} name="applicability" required /></label>
          <label className="block text-xs text-gray-400">Uncertainty<input className={inputClass} name="uncertainty" required /></label>
          <label className="block text-xs text-gray-400">Limitations<input className={inputClass} name="limitations" required /></label>
          <label className="block text-xs text-gray-400">Analysis run IDs
            <select className={inputClass} name="analysisRunIds" multiple required size={Math.min(Math.max(completedRuns.length, 2), 5)}>
              {completedRuns.map((run) => <option key={run.id} value={run.id}>{run.lifecycleStatus} · {run.id}</option>)}
            </select>
          </label>
          <label className="block text-xs text-gray-400">Proposition-matched evidence
            <select className={inputClass} name="evidenceIds" multiple required size={Math.min(Math.max(workspace?.evidence.length ?? 0, 2), 5)}>
              {(workspace?.evidence ?? [])
                .filter((item) => findingPropositionId && isFindingProposition(
                  item.proposition,
                  findingPropositionKind,
                  findingPropositionId,
                ))
                .map((item) => (
                <option key={item.id} value={item.id}>{item.polarity} · {item.id}</option>
                ))}
            </select>
          </label>
          <div className="grid grid-cols-2 gap-2">
            <label className="text-xs text-gray-400">Synthesis polarity
              <select className={inputClass} name="synthesisPolarity">
                <option value="supports">Supports</option><option value="contradicts">Contradicts</option>
                <option value="qualifies">Qualifies</option><option value="neutral">Neutral</option>
              </select>
            </label>
            <label className="text-xs text-gray-400">Synthesis rationale<input className={inputClass} name="synthesisRationale" required /></label>
          </div>
          <button className={buttonClass} disabled={busy || completedRuns.length === 0}>Create draft finding</button>
        </form>
      </div>

      <div className="space-y-2">
        <h4 className="text-sm font-medium text-gray-200">Evidence review</h4>
        {(workspace?.evidence ?? []).length === 0 && <p className="text-xs text-gray-500">No evidence records yet.</p>}
        {(workspace?.evidence ?? []).map((item) => (
          <div key={item.id} className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-[#343741] p-3 text-xs">
            <div className="min-w-0">
              <p className="font-medium text-gray-200">{item.polarity.toUpperCase()} · {item.status}</p>
              <p className="mt-1 break-all text-gray-400">{item.proposition.kind}:{item.proposition.id} ← {item.artifact.kind}:{item.artifact.id}</p>
              <p className="mt-1 text-gray-500">{item.scope} · {item.qualityRationale}</p>
            </div>
            {nextEvidenceStatuses(item.status).map((nextStatus) => (
              <button
                key={nextStatus}
                type="button"
                className={buttonClass}
                disabled={busy}
                onClick={() => void perform(() => analysisService.transitionEvidence({
                  evidenceId: item.id,
                  nextStatus,
                  rationale: `Reviewed as ${nextStatus} in Analysis workspace.`,
                  actorId: null,
                }))}
              >
                Mark {nextStatus}
              </button>
            ))}
          </div>
        ))}
      </div>

      <div className="space-y-2">
        <h4 className="text-sm font-medium text-gray-200">Finding review and synthesis history</h4>
        {(workspace?.findings ?? []).map((finding) => (
          <div key={finding.id} className="rounded-lg border border-[#343741] p-3">
            <div className="flex flex-wrap items-start justify-between gap-2">
              <div>
                <p className="text-sm font-medium text-white">{finding.title} · v{finding.versionNo}</p>
                <p className="mt-1 text-xs text-gray-300">{finding.statement}</p>
                <p className="mt-1 text-[11px] text-gray-500">
                  {finding.conclusionKind} · {finding.status} · proposition {finding.proposition.kind}:{finding.proposition.id}
                </p>
              </div>
              <span className="text-[11px] text-gray-400">SHA-256 {finding.contentSha256}</span>
            </div>
            <div className="mt-2 flex flex-wrap gap-2">
              {nextFindingStatuses(finding.status).map((nextStatus) => (
                <button
                  key={nextStatus}
                  type="button"
                  className={buttonClass}
                  disabled={busy}
                  onClick={() => void perform(() => analysisService.transitionFinding({
                    versionId: finding.id,
                    nextStatus,
                    rationale: `Reviewed as ${nextStatus} in Analysis workspace.`,
                    actorId: null,
                  }))}
                >
                  Mark {nextStatus}
                </button>
              ))}
            </div>
            <details className="mt-2 text-xs text-gray-400">
              <summary className="cursor-pointer">Provenance and revision history</summary>
              <p className="mt-1">Runs: {finding.analysisRunIds.join(", ")}</p>
              <p>Evidence: {finding.evidence.map((link) => `${link.synthesisPolarity}:${link.evidenceId}`).join(", ")}</p>
              <p>Scope: {finding.scope} · Applicability: {finding.applicability} · Uncertainty: {finding.uncertainty}</p>
              <p>Limitations: {finding.limitations}</p>
              <p>Parent: {finding.parentVersionId ?? "initial version"} · Rationale: {finding.changeRationale ?? "initial version"}</p>
              <p>Events: {finding.events.map((item) => `${item.status} (${formatTime(item.occurredAtUs)})`).join(" → ")}</p>
            </details>
          </div>
        ))}
        {(workspace?.findings ?? []).length === 0 && <p className="text-xs text-gray-500">No findings have been synthesized.</p>}
      </div>
    </section>
  );
}
