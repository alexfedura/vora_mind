import React, { useEffect, useMemo, useState } from "react";
import { AlertCircle, GitBranch, Link2, RefreshCw, ShieldCheck } from "lucide-react";
import { knowledgeTransferService } from "@/mind/services/knowledgeTransferService";
import type {
  DownstreamUseRef,
  KnowledgeVersionKind,
  TransferAccessScope,
  TransferArtifactRef,
  TransferEligibilityDto,
  TransferLineageRelation,
  TransferPurpose,
  TransferWorkspaceDto,
} from "@/types";

const TRANSFER_PURPOSES: TransferPurpose[] = [
  "research",
  "learning",
  "commercial",
  "public_distribution",
];
const fieldClass =
  "w-full rounded-lg border border-[#343741] bg-[#101114] px-3 py-2 text-sm text-white outline-none focus:border-[#e8ad55]";
const buttonClass =
  "rounded-lg border border-[#343741] px-3 py-2 text-xs font-medium text-gray-200 hover:bg-[#202229] disabled:opacity-40";
const primaryButtonClass =
  "rounded-lg bg-[#e8ad55] px-3 py-2 text-xs font-semibold text-[#17130d] disabled:opacity-40";

function artifactKey(artifact: TransferArtifactRef): string {
  return `${artifact.kind}:${artifact.versionId}`;
}

function artifactLabel(artifact: TransferArtifactRef, title: string, versionNo: number): string {
  return `${title} · v${versionNo} · ${artifact.kind.replace(/_/g, " ")}`;
}

function isTransferPurpose(value: string): value is TransferPurpose {
  return value === "research"
    || value === "learning"
    || value === "commercial"
    || value === "public_distribution";
}

function parseTransferPurpose(value: string): TransferPurpose {
  if (!isTransferPurpose(value)) throw new Error(`Unsupported transfer purpose: ${value}`);
  return value;
}

function parseAccessScope(value: string): TransferAccessScope {
  if (value === "workspace" || value === "selected_projects") return value;
  throw new Error(`Unsupported transfer access scope: ${value}`);
}

function parseLineageRelation(value: string): TransferLineageRelation {
  if (value === "adopted" || value === "based_on" || value === "derived_from") return value;
  throw new Error(`Unsupported lineage relation: ${value}`);
}

function parseDownstreamKind(value: string): DownstreamUseRef["kind"] {
  switch (value) {
    case "analysis_run":
    case "experiment_run":
    case "model_evaluation":
    case "finding_version":
    case "concept_version":
    case "principle_version":
    case "framework_version":
    case "model_version":
      return value;
    default:
      throw new Error(`Unsupported downstream-use kind: ${value}`);
  }
}

function parseKnowledgeVersionKind(value: string): KnowledgeVersionKind {
  if (value === "concept" || value === "principle") return value;
  throw new Error(`Unsupported knowledge version kind: ${value}`);
}

function isArtifactUseKind(
  kind: DownstreamUseRef["kind"],
): kind is TransferArtifactRef["kind"] {
  return kind.endsWith("_version");
}

function downstreamReference(
  kind: DownstreamUseRef["kind"],
  recordId: string,
): DownstreamUseRef {
  switch (kind) {
    case "model_evaluation": return { kind, id: recordId };
    case "analysis_run": return { kind, id: recordId };
    case "experiment_run": return { kind, id: recordId };
    default: {
      throw new Error("Select an exact downstream artifact version.");
    }
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

export const KnowledgeTransferWorkspacePanel: React.FC = () => {
  const [workspace, setWorkspace] = useState<TransferWorkspaceDto | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [selectedArtifactKey, setSelectedArtifactKey] = useState("");
  const [selectedOriginId, setSelectedOriginId] = useState("");
  const [selectedEligibilityId, setSelectedEligibilityId] = useState("");
  const [selectedTransferId, setSelectedTransferId] = useState("");
  const [selectedProjectId, setSelectedProjectId] = useState("");
  const [selectedPurpose, setSelectedPurpose] = useState<TransferPurpose>("research");
  const [scope, setScope] = useState<TransferAccessScope>("workspace");
  const [purposes, setPurposes] = useState<TransferPurpose[]>(["research"]);
  const [attributionText, setAttributionText] = useState("");
  const [previewOriginId, setPreviewOriginId] = useState("");
  const [preview, setPreview] = useState<Awaited<
    ReturnType<typeof knowledgeTransferService.getProvenancePreview>
  > | null>(null);
  const [usageKind, setUsageKind] = useState<DownstreamUseRef["kind"]>("analysis_run");
  const [knowledgeKind, setKnowledgeKind] = useState<KnowledgeVersionKind>("concept");

  const loadWorkspace = async (): Promise<void> => {
    setLoading(true);
    setError(null);
    try {
      const result = await knowledgeTransferService.getWorkspace();
      setWorkspace(result);
      setSelectedProjectId((current) =>
        result.projects.some((project) => project.id === current)
          ? current
          : result.projects[0]?.id ?? "",
      );
      setSelectedOriginId((current) =>
        result.origins.some((origin) => origin.id === current)
          ? current
          : result.origins[0]?.id ?? "",
      );
      setSelectedEligibilityId((current) =>
        result.eligibilities.some((eligibility) => eligibility.id === current)
          ? current
          : result.eligibilities.find((eligibility) => eligibility.state === "eligible")?.id ?? "",
      );
      setSelectedTransferId((current) =>
        result.transfers.some((transfer) => transfer.id === current)
          ? current
          : result.transfers.find((transfer) => transfer.state === "accepted")?.id ?? "",
      );
      if (!selectedArtifactKey && result.artifacts[0]) {
        setSelectedArtifactKey(artifactKey(result.artifacts[0].artifact));
      }
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : String(loadError));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadWorkspace();
  }, []);

  const selectedArtifact = useMemo(
    () => workspace?.artifacts.find((item) => artifactKey(item.artifact) === selectedArtifactKey),
    [workspace, selectedArtifactKey],
  );
  const selectedOrigin = workspace?.origins.find((item) => item.id === selectedOriginId);
  const selectedEligibility = workspace?.eligibilities.find(
    (item) => item.id === selectedEligibilityId,
  );
  const selectedTransfer = workspace?.transfers.find((item) => item.id === selectedTransferId);
  const adaptationCandidates = (workspace?.artifacts ?? []).filter((item) =>
    selectedTransfer
    && item.ownerProjectId === selectedTransfer.receivingProjectId
    && item.artifact.kind === selectedTransfer.origin.artifact.kind
    && item.artifact.versionId !== selectedTransfer.origin.artifact.versionId
    && entityId(item.artifact) !== entityId(selectedTransfer.origin.artifact),
  );

  const perform = async <T,>(
    action: () => Promise<T>,
    successMessage: string,
    afterSuccess?: (result: T) => void,
  ): Promise<void> => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const result = await action();
      setNotice(successMessage);
      await loadWorkspace();
      afterSuccess?.(result);
    } catch (actionError) {
      setError(actionError instanceof Error ? actionError.message : String(actionError));
    } finally {
      setBusy(false);
    }
  };

  const handleCreateOrigin = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!selectedArtifact) return;
    const form = new FormData(event.currentTarget);
    await perform(
      () => knowledgeTransferService.createOrigin({
        artifact: selectedArtifact.artifact,
        originProjectId: String(form.get("originProjectId") ?? ""),
        rationale: String(form.get("rationale") ?? ""),
        recordedBy: optionalString(form.get("recordedBy")),
      }),
      "Origin assertion recorded for this exact version.",
      (origin) => setSelectedOriginId(origin.id),
    );
  };

  const handleCreateEligibility = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!selectedOrigin) return;
    const form = new FormData(event.currentTarget);
    await perform(
      () => knowledgeTransferService.createEligibility({
        originId: selectedOrigin.id,
        licenseIdentifier: String(form.get("licenseIdentifier") ?? ""),
        licenseUri: optionalString(form.get("licenseUri")),
        accessScope: scope,
        permittedPurposes: purposes,
        allowedProjectIds: scope === "selected_projects"
          ? [String(form.get("allowedProjectId") ?? "")]
          : [],
        attributionText: String(form.get("attributionText") ?? ""),
        permissionBasis: String(form.get("permissionBasis") ?? ""),
        permissionGranted: form.get("permissionGranted") === "on",
        reviewedBy: String(form.get("reviewedBy") ?? ""),
      }),
      "A new immutable eligibility snapshot was recorded.",
      (eligibility) => setSelectedEligibilityId(eligibility.id),
    );
  };

  const handleAdopt = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!selectedEligibility) return;
    const form = new FormData(event.currentTarget);
    await perform(
      () => knowledgeTransferService.adopt({
        eligibilityId: selectedEligibility.id,
        receivingProjectId: String(form.get("receivingProjectId") ?? ""),
        intendedPurpose: parseTransferPurpose(String(form.get("intendedPurpose") ?? "research")),
        attributionText: String(form.get("attributionText") ?? ""),
        context: String(form.get("context") ?? ""),
        requestedBy: optionalString(form.get("requestedBy")),
      }),
      "Exact artifact version adopted with attribution and lineage.",
    );
  };

  const handleCreateAdaptation = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!selectedTransfer) return;
    const form = new FormData(event.currentTarget);
    const targetKey = String(form.get("targetArtifact") ?? "");
    const target = adaptationCandidates.find((item) => artifactKey(item.artifact) === targetKey);
    if (!target) {
      setError("Select an exact artifact version created in the receiving project.");
      return;
    }
    await perform(
      () => knowledgeTransferService.createAdaptation({
        transferId: selectedTransfer.id,
        artifact: target.artifact,
        relationKind: parseLineageRelation(String(form.get("relationKind") ?? "derived_from")),
        rationale: String(form.get("rationale") ?? ""),
        adaptedBy: optionalString(form.get("adaptedBy")),
      }),
      "Adaptation and upstream lineage recorded.",
    );
  };

  const handleRecordUsage = async (event: React.FormEvent<HTMLFormElement>): Promise<void> => {
    event.preventDefault();
    if (!selectedTransfer) return;
    const form = new FormData(event.currentTarget);
    let reference: DownstreamUseRef;
    if (isArtifactUseKind(usageKind)) {
      const selected = workspace?.artifacts.find(
        (item) =>
          artifactKey(item.artifact) === String(form.get("artifact") ?? "")
          && item.artifact.kind === usageKind,
      );
      if (!selected) {
        setError("Select an exact downstream artifact version.");
        return;
      }
      reference = selected.artifact;
    } else {
      reference = downstreamReference(usageKind, String(form.get("recordId") ?? ""));
    }
    await perform(
      () => knowledgeTransferService.recordUsage({
        transferId: selectedTransfer.id,
        reference,
        useContext: String(form.get("useContext") ?? ""),
        recordedBy: optionalString(form.get("recordedBy")),
      }),
      "Downstream use recorded.",
    );
  };

  const handleCreateKnowledgeVersion = async (
    event: React.FormEvent<HTMLFormElement>,
  ): Promise<void> => {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    const request = {
      conceptId: null,
      principleId: null,
      projectId: String(form.get("projectId") ?? ""),
      parentVersionId: null,
      title: String(form.get("title") ?? ""),
      definition: String(form.get("content") ?? ""),
      statement: String(form.get("content") ?? ""),
      rationale: String(form.get("rationale") ?? ""),
      createdBy: optionalString(form.get("createdBy")),
    };
    const createVersion = knowledgeKind === "concept"
      ? () => knowledgeTransferService.createConceptVersion({
        conceptId: request.conceptId,
        projectId: request.projectId,
        parentVersionId: request.parentVersionId,
        title: request.title,
        definition: request.definition,
        rationale: request.rationale,
        createdBy: request.createdBy,
      })
      : () => knowledgeTransferService.createPrincipleVersion({
        principleId: request.principleId,
        projectId: request.projectId,
        parentVersionId: request.parentVersionId,
        title: request.title,
        statement: request.statement,
        rationale: request.rationale,
        createdBy: request.createdBy,
      });
    await perform(createVersion, `${knowledgeKind} version created as a draft.`);
  };

  const previewOrigin = async (originId: string): Promise<void> => {
    setError(null);
    setPreview(null);
    setPreviewOriginId(originId);
    try {
      setPreview(await knowledgeTransferService.getProvenancePreview(originId));
    } catch (previewError) {
      setError(previewError instanceof Error ? previewError.message : String(previewError));
    }
  };

  const reviewKnowledgeVersion = async (
    kind: KnowledgeVersionKind,
    versionId: string,
  ): Promise<void> => {
    await perform(
      () => knowledgeTransferService.reviewKnowledgeVersion({
        versionId,
        kind,
        status: "reviewed",
        rationale: "Reviewed for extraction/content quality; this does not assert scientific truth.",
        actorId: null,
      }),
      "Knowledge version reviewed. Review does not establish scientific truth.",
    );
  };

  const changeEligibility = async (
    eligibility: TransferEligibilityDto,
    state: "restricted" | "withdrawn",
  ): Promise<void> => {
    const rationale = window.prompt(`Why is future adoption being ${state}?`);
    if (!rationale?.trim()) return;
    await perform(
      () => knowledgeTransferService.changeEligibility({
        eligibilityId: eligibility.id,
        state,
        rationale,
        actorId: null,
      }),
      `Future adoption ${state}; historical adoptions remain intact.`,
    );
  };

  return (
    <section className="space-y-4 rounded-xl border border-[#343741] bg-[#17191e] p-4">
      <header className="flex items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2 text-white">
            <GitBranch size={16} className="text-[#e8ad55]" />
            <h2 className="text-sm font-semibold">Cross-project knowledge transfer</h2>
          </div>
          <p className="mt-1 text-xs text-gray-400">
            Adopt exact versions with explicit rights, attribution, and bidirectional lineage.
          </p>
        </div>
        <button type="button" className={buttonClass} onClick={() => void loadWorkspace()} disabled={loading || busy}>
          <RefreshCw size={13} className={loading ? "animate-spin" : ""} /> Refresh
        </button>
      </header>

      {error && (
        <div role="alert" className="flex items-start gap-2 rounded-lg border border-red-500/30 bg-red-500/10 p-3 text-xs text-red-200">
          <AlertCircle size={14} className="mt-0.5 shrink-0" /> {error}
        </div>
      )}
      {notice && <p role="status" className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 p-3 text-xs text-emerald-200">{notice}</p>}

      {loading && !workspace ? (
        <p className="text-xs text-gray-400">Loading transfer records…</p>
      ) : (
        <>
          <div className="grid gap-4 xl:grid-cols-2">
            <div className="space-y-3 rounded-lg border border-[#292c34] p-3">
              <h3 className="text-xs font-semibold uppercase tracking-wide text-gray-300">Reusable knowledge</h3>
              <Field label="Exact transferable artifact version">
                <select
                  className={fieldClass}
                  value={selectedArtifactKey}
                  onChange={(event) => setSelectedArtifactKey(event.target.value)}
                >
                  <option value="">Select an artifact</option>
                  {(workspace?.artifacts ?? []).map((item) => (
                    <option key={artifactKey(item.artifact)} value={artifactKey(item.artifact)}>
                      {artifactLabel(item.artifact, item.title, item.versionNo)} · {item.lifecycleStatus}
                    </option>
                  ))}
                </select>
              </Field>
              {selectedArtifact && (
                <div className="rounded-lg bg-[#101114] p-3 text-xs text-gray-300">
                  <p>{selectedArtifact.title} v{selectedArtifact.versionNo}</p>
                  <p className="mt-1 text-gray-500">State: {selectedArtifact.lifecycleStatus} · origin: {selectedArtifact.originId ? "asserted" : "not yet asserted"} · exact version {selectedArtifact.artifact.versionId}</p>
                  {selectedArtifact.originId && (
                    <button type="button" className={`${buttonClass} mt-2`} onClick={() => void previewOrigin(selectedArtifact.originId ?? "")}>
                      Preview complete provenance
                    </button>
                  )}
                </div>
              )}
              <form key={selectedArtifactKey} onSubmit={(event) => void handleCreateOrigin(event)} className="space-y-2 border-t border-[#292c34] pt-3">
                <Field label="Origin project">
                  <select className={fieldClass} name="originProjectId" defaultValue={selectedArtifact?.ownerProjectId ?? ""} required>
                    <option value="">Select project</option>
                    {(workspace?.projects ?? []).map((project) => <option key={project.id} value={project.id}>{project.title}</option>)}
                  </select>
                </Field>
                <Field label="Origin rationale">
                  <input className={fieldClass} name="rationale" placeholder="How origin was verified" required />
                </Field>
                <Field label="Recorded by"><input className={fieldClass} name="recordedBy" /></Field>
                <button className={primaryButtonClass} disabled={busy || !selectedArtifact}>Assert origin for exact version</button>
              </form>

              <form onSubmit={(event) => void handleCreateEligibility(event)} className="space-y-2 border-t border-[#292c34] pt-3">
                <Field label="Origin whose rights will be reviewed">
                  <select className={fieldClass} value={selectedOriginId} onChange={(event) => setSelectedOriginId(event.target.value)} required>
                    <option value="">Select origin assertion</option>
                    {(workspace?.origins ?? []).map((origin) => <option key={origin.id} value={origin.id}>{origin.originProjectTitle} · {origin.artifact.kind} · {origin.artifact.versionId.slice(0, 8)}</option>)}
                  </select>
                </Field>
                <div className="grid gap-2 sm:grid-cols-2">
                  <Field label="License identifier"><input className={fieldClass} name="licenseIdentifier" placeholder="License or rights basis" required /></Field>
                  <Field label="License URI"><input className={fieldClass} name="licenseUri" type="url" /></Field>
                </div>
                <Field label="Access scope">
                  <select className={fieldClass} value={scope} onChange={(event) => setScope(parseAccessScope(event.target.value))}>
                    <option value="workspace">Workspace-wide</option>
                    <option value="selected_projects">Selected projects</option>
                  </select>
                </Field>
                {scope === "selected_projects" && (
                  <Field label="Allowed receiving project">
                    <select className={fieldClass} name="allowedProjectId" required>
                      <option value="">Select a recipient</option>
                      {(workspace?.projects ?? []).filter((project) => project.id !== selectedOrigin?.originProjectId).map((project) => <option key={project.id} value={project.id}>{project.title}</option>)}
                    </select>
                  </Field>
                )}
                <div className="flex flex-wrap gap-3 text-xs text-gray-300">
                  {TRANSFER_PURPOSES.map((purpose) => (
                    <label key={purpose} className="flex items-center gap-1.5">
                      <input
                        type="checkbox"
                        checked={purposes.includes(purpose)}
                        onChange={(event) => setPurposes((current) =>
                          event.target.checked
                            ? [...current, purpose]
                            : current.filter((value) => value !== purpose),
                        )}
                      />
                      {purpose.replace(/_/g, " ")}
                    </label>
                  ))}
                </div>
                <Field label="Attribution text"><textarea className={fieldClass} name="attributionText" rows={2} required /></Field>
                <Field label="Permission basis"><textarea className={fieldClass} name="permissionBasis" rows={2} required /></Field>
                <Field label="Reviewed by"><input className={fieldClass} name="reviewedBy" required /></Field>
                <label className="flex items-start gap-2 text-xs text-amber-200">
                  <input className="mt-0.5" name="permissionGranted" type="checkbox" required />
                  I verified and recorded permission for the selected purposes and scope.
                </label>
                <button className={primaryButtonClass} disabled={busy || !selectedOriginId || purposes.length === 0}>
                  Record rights and eligibility snapshot
                </button>
              </form>
            </div>

            <div className="space-y-3 rounded-lg border border-[#292c34] p-3">
              <h3 className="text-xs font-semibold uppercase tracking-wide text-gray-300">Adoption and use</h3>
              <form onSubmit={(event) => void handleAdopt(event)} className="space-y-2">
                <Field label="Eligible exact version">
                  <select
                    className={fieldClass}
                    value={selectedEligibilityId}
                    onChange={(event) => {
                      const eligibility = workspace?.eligibilities.find((item) => item.id === event.target.value);
                      setSelectedEligibilityId(event.target.value);
                      setAttributionText(eligibility?.attributionText ?? "");
                      setSelectedPurpose(eligibility?.permittedPurposes[0] ?? "research");
                        const projects = workspace?.projects ?? [];
                        const validProject = projects.some((project) =>
                          project.id === selectedProjectId
                          && project.id !== eligibility?.origin.originProjectId
                          && (
                            eligibility?.accessScope === "workspace"
                            || eligibility?.allowedProjectIds.includes(project.id)
                          ),
                        );
                        if (!validProject) {
                          const nextProject = projects.find((project) =>
                            project.id !== eligibility?.origin.originProjectId
                            && (
                              eligibility?.accessScope === "workspace"
                              || eligibility?.allowedProjectIds.includes(project.id)
                            ),
                          );
                          setSelectedProjectId(nextProject?.id ?? "");
                        }
                    }}
                    required
                  >
                    <option value="">Select current eligibility</option>
                    {(workspace?.eligibilities ?? []).filter((item) => item.state === "eligible").map((item) => (
                      <option key={item.id} value={item.id}>{item.origin.originProjectTitle} · {item.origin.artifact.kind} · {item.licenseIdentifier}</option>
                    ))}
                  </select>
                </Field>
                <Field label="Receiving project">
                  <select className={fieldClass} name="receivingProjectId" value={selectedProjectId} onChange={(event) => setSelectedProjectId(event.target.value)} required>
                    <option value="">Select receiving project</option>
                    {(workspace?.projects ?? []).filter((project) => project.id !== selectedEligibility?.origin.originProjectId).map((project) => <option key={project.id} value={project.id}>{project.title}</option>)}
                  </select>
                </Field>
                <Field label="Purpose">
                  <select className={fieldClass} name="intendedPurpose" value={selectedPurpose} onChange={(event) => setSelectedPurpose(parseTransferPurpose(event.target.value))} required>
                    {(selectedEligibility?.permittedPurposes ?? []).map((purpose) => <option key={purpose} value={purpose}>{purpose.replace(/_/g, " ")}</option>)}
                  </select>
                </Field>
                <Field label="Attribution"><textarea className={fieldClass} name="attributionText" value={attributionText} onChange={(event) => setAttributionText(event.target.value)} rows={2} required /></Field>
                <Field label="Transfer context"><textarea className={fieldClass} name="context" rows={2} required /></Field>
                <Field label="Adopted by"><input className={fieldClass} name="requestedBy" /></Field>
                <button className={primaryButtonClass} disabled={busy || !selectedEligibilityId}>Adopt exact version</button>
              </form>

              <form onSubmit={(event) => void handleCreateAdaptation(event)} className="space-y-2 border-t border-[#292c34] pt-3">
                <Field label="Accepted transfer">
                  <select className={fieldClass} value={selectedTransferId} onChange={(event) => setSelectedTransferId(event.target.value)} required>
                    <option value="">Select accepted transfer</option>
                    {(workspace?.transfers ?? []).filter((transfer) => transfer.state === "accepted").map((transfer) => <option key={transfer.id} value={transfer.id}>{transfer.receivingProjectTitle} ← {transfer.origin.originProjectTitle} · {transfer.origin.artifact.versionId.slice(0, 8)}</option>)}
                  </select>
                </Field>
                <Field label="Distinct artifact version in receiving project">
                  <select className={fieldClass} name="targetArtifact" required>
                    <option value="">Select a version of the same kind</option>
                    {adaptationCandidates.map((item) => <option key={artifactKey(item.artifact)} value={artifactKey(item.artifact)}>{artifactLabel(item.artifact, item.title, item.versionNo)}</option>)}
                  </select>
                </Field>
                <div className="grid gap-2 sm:grid-cols-2">
                  <Field label="Lineage relation">
                    <select className={fieldClass} name="relationKind">
                      <option value="derived_from">Derived from</option>
                      <option value="based_on">Based on</option>
                    </select>
                  </Field>
                  <Field label="Adapted by"><input className={fieldClass} name="adaptedBy" /></Field>
                </div>
                <Field label="Adaptation rationale"><input className={fieldClass} name="rationale" required /></Field>
                <button className={buttonClass} disabled={busy || !selectedTransferId || adaptationCandidates.length === 0}>Record adaptation and lineage</button>
              </form>

              <form onSubmit={(event) => void handleRecordUsage(event)} className="space-y-2 border-t border-[#292c34] pt-3">
                <Field label="Downstream use type">
                  <select className={fieldClass} value={usageKind} onChange={(event) => setUsageKind(parseDownstreamKind(event.target.value))}>
                    <option value="analysis_run">Analysis run</option>
                    <option value="experiment_run">Experiment run</option>
                    <option value="model_evaluation">Model evaluation</option>
                    <option value="finding_version">Finding version</option>
                    <option value="concept_version">Concept version</option>
                    <option value="principle_version">Principle version</option>
                    <option value="framework_version">Framework version</option>
                    <option value="model_version">Model version</option>
                  </select>
                </Field>
                {isArtifactUseKind(usageKind) ? (
                  <Field label="Exact downstream artifact version">
                    <select className={fieldClass} name="artifact" required>
                      <option value="">Select artifact version</option>
                      {(workspace?.artifacts ?? []).filter((item) => item.ownerProjectId === selectedTransfer?.receivingProjectId && item.artifact.kind === usageKind).map((item) => <option key={artifactKey(item.artifact)} value={artifactKey(item.artifact)}>{artifactLabel(item.artifact, item.title, item.versionNo)}</option>)}
                    </select>
                  </Field>
                ) : (
                  <Field label="Downstream record ID"><input className={fieldClass} name="recordId" required /></Field>
                )}
                <Field label="Use context"><input className={fieldClass} name="useContext" required /></Field>
                <Field label="Recorded by"><input className={fieldClass} name="recordedBy" /></Field>
                <button className={buttonClass} disabled={busy || !selectedTransferId}>Record downstream use</button>
              </form>
            </div>
          </div>

          <div className="grid gap-4 xl:grid-cols-2">
            <div className="space-y-3 rounded-lg border border-[#292c34] p-3">
              <h3 className="text-xs font-semibold uppercase tracking-wide text-gray-300">Versioned concepts and principles</h3>
              <form onSubmit={(event) => void handleCreateKnowledgeVersion(event)} className="space-y-2">
                <div className="grid gap-2 sm:grid-cols-2">
                  <Field label="Knowledge type">
                    <select className={fieldClass} value={knowledgeKind} onChange={(event) => setKnowledgeKind(parseKnowledgeVersionKind(event.target.value))}>
                      <option value="concept">Concept</option>
                      <option value="principle">Principle</option>
                    </select>
                  </Field>
                  <Field label="Origin project">
                    <select className={fieldClass} name="projectId" required>
                      {(workspace?.projects ?? []).map((project) => <option key={project.id} value={project.id}>{project.title}</option>)}
                    </select>
                  </Field>
                </div>
                <Field label="Title"><input className={fieldClass} name="title" required /></Field>
                <Field label={knowledgeKind === "concept" ? "Definition" : "Statement"}><textarea className={fieldClass} name="content" rows={2} required /></Field>
                <div className="grid gap-2 sm:grid-cols-2">
                  <Field label="Version rationale"><input className={fieldClass} name="rationale" required /></Field>
                  <Field label="Created by"><input className={fieldClass} name="createdBy" /></Field>
                </div>
                <button className={buttonClass} disabled={busy || !workspace?.projects.length}>Create immutable draft version</button>
              </form>
              <div className="space-y-2">
                {(workspace?.knowledgeVersions ?? []).map((version) => (
                  <div key={version.versionId} className="rounded-lg bg-[#101114] p-3 text-xs text-gray-300">
                    <p className="font-medium text-white">{version.kind}: {version.title} · v{version.versionId.slice(0, 8)} · {version.status}</p>
                    <p className="mt-1 text-gray-400">{version.content}</p>
                    {version.status === "draft" && (
                      <button type="button" className={`${buttonClass} mt-2`} disabled={busy} onClick={() => void reviewKnowledgeVersion(version.kind, version.versionId)}>
                        Review content quality (not scientific truth)
                      </button>
                    )}
                  </div>
                ))}
              </div>
            </div>

            <div className="space-y-3 rounded-lg border border-[#292c34] p-3">
              <h3 className="flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-gray-300">
                <ShieldCheck size={14} /> Eligibility and history
              </h3>
              <div className="space-y-2">
                {(workspace?.eligibilities ?? []).map((eligibility) => (
                  <div key={eligibility.id} className="rounded-lg bg-[#101114] p-3 text-xs text-gray-300">
                    <p className="font-medium text-white">{eligibility.origin.originProjectTitle} → {eligibility.licenseIdentifier} · {eligibility.state}</p>
                    <p className="mt-1 text-gray-400">{eligibility.origin.artifact.kind} {eligibility.origin.artifact.versionId} · {eligibility.accessScope.replace(/_/g, " ")} · reviewer {eligibility.reviewedBy}</p>
                    <p className="mt-1 text-gray-500">{eligibility.stateRationale}</p>
                    <div className="mt-2 flex flex-wrap gap-2">
                      <button type="button" className={buttonClass} onClick={() => void previewOrigin(eligibility.origin.id)}>Provenance preview</button>
                      {eligibility.state === "eligible" && (
                        <>
                          <button type="button" className={buttonClass} disabled={busy} onClick={() => void changeEligibility(eligibility, "restricted")}>Restrict future adoption</button>
                          <button type="button" className={buttonClass} disabled={busy} onClick={() => void changeEligibility(eligibility, "withdrawn")}>Withdraw eligibility</button>
                        </>
                      )}
                    </div>
                  </div>
                ))}
              </div>
              {preview && previewOriginId && (
                <div className="rounded-lg border border-[#454953] bg-[#101114] p-3 text-xs text-gray-300">
                  <div className="flex items-center justify-between gap-2">
                    <h4 className="font-semibold text-white">{preview.title} · v{preview.versionNo} · {preview.lifecycleStatus}</h4>
                    <button type="button" className={buttonClass} onClick={() => setPreview(null)}>Close</button>
                  </div>
                  <p className="mt-2"><strong>Scope:</strong> {preview.scope}</p>
                  <p className="mt-1"><strong>Applicability:</strong> {preview.applicability}</p>
                  <p className="mt-1"><strong>Uncertainty:</strong> {preview.uncertainty || "Not recorded"}</p>
                  <p className="mt-1"><strong>Limitations:</strong> {preview.limitations || "Not recorded"}</p>
                  <p className="mt-2 font-semibold">Validation scope</p>
                  {preview.validationScope.length ? preview.validationScope.map((scopeItem) => <p key={scopeItem}>• {scopeItem}</p>) : <p className="text-amber-300">No validation scope recorded; this is not evidence of validation.</p>}
                  <p className="mt-2 font-semibold">Evidence and provenance</p>
                  {preview.provenance.length ? preview.provenance.map((record) => (
                    <p key={`${record.recordKind}:${record.recordId}`} className="mt-1">
                      <Link2 size={12} className="mr-1 inline" />{record.label} · {record.relationship} · {record.details}
                    </p>
                  )) : <p className="text-gray-500">No linked provenance records were found.</p>}
                </div>
              )}
            </div>
          </div>

          <div className="grid gap-4 xl:grid-cols-2">
            <HistoryList title="Transfers and attribution" items={(workspace?.transfers ?? []).map((transfer) => (
              <div key={transfer.id} className="rounded-lg bg-[#101114] p-3 text-xs text-gray-300">
                <p className="font-medium text-white">{transfer.origin.originProjectTitle} → {transfer.receivingProjectTitle} · {transfer.state}</p>
                <p className="mt-1">{transfer.origin.artifact.kind} {transfer.origin.artifact.versionId}</p>
                <p className="mt-1">Purpose: {transfer.intendedPurpose} · Attribution: {transfer.attributionText}</p>
                <p className="mt-1 text-gray-500">{transfer.context}</p>
              </div>
            ))} />
            <HistoryList title="Lineage and downstream use" items={[
              ...(workspace?.lineage ?? []).map((item) => (
                <div key={item.id} className="rounded-lg bg-[#101114] p-3 text-xs text-gray-300">
                  <p className="flex items-center gap-1 font-medium text-white"><GitBranch size={12} />{item.relationKind}: {item.originProjectId} → {item.receivingProjectId}</p>
                  <p className="mt-1">Origin {item.origin.kind} {item.origin.versionId}{item.adaptation ? ` → ${item.adaptation.kind} ${item.adaptation.versionId}` : ""}</p>
                  <p className="mt-1 text-gray-500">{item.rationale}</p>
                </div>
              )),
              ...(workspace?.downstreamUsage ?? []).map((item) => (
                <div key={item.id} className="rounded-lg bg-[#101114] p-3 text-xs text-gray-300">
                  <p className="font-medium text-white">Downstream use · {item.reference.kind}</p>
                  <p className="mt-1">Project {item.projectId} · record {referenceId(item.reference)}</p>
                  <p className="mt-1 text-gray-500">{item.useContext}</p>
                </div>
              )),
            ]} />
          </div>
        </>
      )}
    </section>
  );
};

function entityId(artifact: TransferArtifactRef): string {
  switch (artifact.kind) {
    case "finding_version": return artifact.findingId;
    case "concept_version": return artifact.conceptId;
    case "principle_version": return artifact.principleId;
    case "framework_version": return artifact.frameworkId;
    case "model_version": return artifact.modelId;
  }
}

function referenceId(reference: DownstreamUseRef): string {
  return "versionId" in reference ? reference.versionId : reference.id;
}

function optionalString(value: FormDataEntryValue | null): string | null {
  const normalized = String(value ?? "").trim();
  return normalized ? normalized : null;
}

function HistoryList({
  title,
  items,
}: {
  title: string;
  items: React.ReactNode[];
}): React.JSX.Element {
  return (
    <div className="space-y-2 rounded-lg border border-[#292c34] p-3">
      <h3 className="text-xs font-semibold uppercase tracking-wide text-gray-300">{title}</h3>
      {items.length ? items : <p className="text-xs text-gray-500">No records yet.</p>}
    </div>
  );
}
