import type {
  AnalysisWorkspaceDto,
  ExperimentWorkspaceDto,
  ModelWorkspaceDto,
  TransferArtifactRef,
  TransferWorkspaceDto,
} from "@/types";
import type { ProjectKnowledgeOverview } from "@/shared/layout/projectKnowledgeModel";

export type ProjectLifecycleLoadStatus = "idle" | "loading" | "loaded" | "error";
export type ProjectLifecycleAvailability =
  | "available"
  | "outside_project"
  | "unavailable"
  | "not_verified";

export interface ProjectLifecycleDomain<T> {
  status: ProjectLifecycleLoadStatus;
  data: T | null;
  error: string | null;
}

export interface ProjectLifecycleEndpoint {
  kind: string;
  id: string;
  label: string;
  projectId: string | null;
  status: string | null;
  availability: ProjectLifecycleAvailability;
  href: string | null;
}

export interface ProjectLifecycleConnection {
  id: string;
  category: string;
  from: ProjectLifecycleEndpoint;
  relationship: string;
  to: ProjectLifecycleEndpoint;
  recordKind: string;
  recordId: string;
  note: string | null;
}

export interface ProjectLifecycleInput {
  projectId: string;
  overview: ProjectKnowledgeOverview;
  analysis: ProjectLifecycleDomain<AnalysisWorkspaceDto>;
  experiment: ProjectLifecycleDomain<ExperimentWorkspaceDto>;
  model: ProjectLifecycleDomain<ModelWorkspaceDto>;
  transfer: ProjectLifecycleDomain<TransferWorkspaceDto>;
}

const anchor = (kind: string, id: string): string =>
  `#project-lifecycle-${kind}-${encodeURIComponent(id)}`;

function key(kind: string, id: string): string {
  return `${kind}\u0000${id}`;
}

function projectHref(
  kind: string,
  id: string,
  projectId: string | null,
): string | null {
  switch (kind) {
    case "research_question":
    case "research_hypothesis":
    case "hypothesis_version":
    case "research_observation":
    case "research_pattern":
    case "research_design":
    case "research_literature_review":
    case "framework_adoption":
      return anchor(kind, id);
    case "experiment_design":
    case "experiment_run":
    case "dataset_version":
    case "dataset_transformation":
      return anchor(kind, id);
    case "analysis_definition":
    case "analysis_method":
    case "analysis_metric":
    case "analysis_metric_result":
    case "analysis_run":
    case "analysis_output":
    case "analysis_evidence":
    case "analysis_input":
    case "finding_version":
      return anchor(kind, id);
    case "model_version":
    case "model_evidence":
    case "model_evaluation":
    case "model_validation":
    case "model_replication":
      return anchor(kind, id);
    case "transfer_origin":
    case "transfer_eligibility":
    case "knowledge_transfer":
    case "transfer_adaptation":
    case "downstream_usage":
    case "transfer_lineage":
      return anchor(kind, id);
    case "framework_version":
      return anchor(kind, id);
    case "concept_version":
    case "principle_version":
      return anchor(kind, id);
    case "source_claim": {
      if (!projectId) return "/sources";
      const sourceParams = new URLSearchParams({ sourceClaimId: id });
      return `/sources?${sourceParams.toString()}`;
    }
    default:
      return null;
  }
}

function referenceHref(
  kind: string,
  id: string,
  overview: ProjectKnowledgeOverview,
): string | null {
  if (kind === "framework_version") {
    return overview.adoptions.some((adoption) => adoption.frameworkVersionId === id)
      ? anchor(kind, id)
      : "#project-knowledge-frameworks";
  }
  if (kind === "source_claim") {
    const reference = overview.references.find((item) => item.sourceClaimId === id);
    if (!reference?.sourceId) return "/sources";
    const params = new URLSearchParams({ sourceId: reference.sourceId });
    if (reference.sourceVersionId) params.set("sourceVersionId", reference.sourceVersionId);
    if (reference.sourceLocationId) params.set("sourceLocationId", reference.sourceLocationId);
    params.set("sourceClaimId", id);
    return `/sources?${params.toString()}`;
  }
  if (kind === "source_version_segment") {
    const reference = overview.references.find((item) => item.sourceLocationId === id);
    if (!reference?.sourceId) return null;
    const params = new URLSearchParams({ sourceId: reference.sourceId });
    if (reference.sourceVersionId) params.set("sourceVersionId", reference.sourceVersionId);
    params.set("sourceLocationId", id);
    return `/sources?${params.toString()}`;
  }
  if (kind === "source_version" || kind === "source_location") {
    const reference = overview.references.find((item) =>
      kind === "source_version"
        ? item.sourceVersionId === id
        : item.sourceLocationId === id,
    );
    if (!reference?.sourceId) return null;
    const params = new URLSearchParams({ sourceId: reference.sourceId });
    if (reference.sourceVersionId) params.set("sourceVersionId", reference.sourceVersionId);
    if (kind === "source_location") params.set("sourceLocationId", id);
    return `/sources?${params.toString()}`;
  }
  return null;
}

function transferArtifactKind(artifact: TransferArtifactRef): string {
  switch (artifact.kind) {
    case "finding_version": return "finding_version";
    case "concept_version": return "concept_version";
    case "principle_version": return "principle_version";
    case "framework_version": return "framework_version";
    case "model_version": return "model_version";
  }
}

function transferArtifactId(artifact: TransferArtifactRef): string {
  return artifact.versionId;
}

function domainStatusFor(
  kind: string,
  input: ProjectLifecycleInput,
): ProjectLifecycleLoadStatus {
  if (kind === "research_question" || kind === "hypothesis_version"
    || kind === "research_hypothesis" || kind === "research_observation"
    || kind === "research_pattern" || kind === "research_design"
    || kind === "research_literature_review" || kind === "framework_adoption"
    || kind === "source_claim" || kind === "source_version_segment") return "loaded";
  if (kind.startsWith("analysis_") || kind === "finding_version") return input.analysis.status;
  if (kind.startsWith("experiment_") || kind === "dataset_version") return input.experiment.status;
  if (kind.startsWith("model_")) return input.model.status;
  if (kind.startsWith("transfer_") || kind === "knowledge_transfer"
    || kind === "downstream_usage" || kind === "concept_version"
    || kind === "principle_version" || kind === "framework_version") return input.transfer.status;
  return "loaded";
}

export function buildProjectLifecycleConnections(
  input: ProjectLifecycleInput,
): ProjectLifecycleConnection[] {
  const { projectId, overview } = input;
  const entities = new Map<string, ProjectLifecycleEndpoint>();
  const connections: ProjectLifecycleConnection[] = [];

  const register = (
    kind: string,
    id: string,
    label: string,
    ownerProjectId: string | null,
    status: string | null,
    href: string | null = projectHref(kind, id, ownerProjectId),
  ): void => {
    if (!id) return;
    entities.set(key(kind, id), {
      kind,
      id,
      label,
      projectId: ownerProjectId,
      status,
      availability: ownerProjectId && ownerProjectId !== projectId
        ? "outside_project"
        : "available",
      href,
    });
  };

  const ref = (
    kind: string,
    id: string,
    label: string,
    ownerProjectId: string | null = null,
  ): ProjectLifecycleEndpoint => {
    const known = entities.get(key(kind, id));
    if (known) return known;
    const externalOwner = ownerProjectId && ownerProjectId !== projectId;
    const domainStatus = domainStatusFor(kind, input);
    const availability: ProjectLifecycleAvailability = externalOwner
      ? "outside_project"
      : domainStatus === "loaded" || domainStatus === "idle"
        ? "unavailable"
        : "not_verified";
    return {
      kind,
      id,
      label,
      projectId: ownerProjectId,
      status: null,
      availability,
      href: referenceHref(kind, id, overview) ?? projectHref(kind, id, ownerProjectId),
    };
  };

  const connect = (
    category: string,
    from: ProjectLifecycleEndpoint,
    relationship: string,
    to: ProjectLifecycleEndpoint,
    recordKind: string,
    recordId: string,
    note: string | null = null,
  ): void => {
    connections.push({
      id: `${recordKind}:${recordId}:${category}:${from.kind}:${from.id}:${relationship}:${to.kind}:${to.id}`,
      category,
      from,
      relationship,
      to,
      recordKind,
      recordId,
      note,
    });
  };

  for (const question of overview.questions) {
    register("research_question", question.id, question.title, projectId, question.lifecycleStatus);
  }
  for (const hypothesis of overview.hypotheses) {
    const latestDefinition = [...hypothesis.definitions].sort((a, b) => b.versionNo - a.versionNo)[0];
    register(
      "research_hypothesis",
      hypothesis.id,
      latestDefinition?.statementH1 ?? `Hypothesis ${hypothesis.id}`,
      hypothesis.projectId,
      hypothesis.lifecycleStatus,
    );
    for (const definition of hypothesis.definitions) {
      register(
        "hypothesis_version",
        definition.id,
        definition.statementH1 || `Hypothesis definition v${definition.versionNo}`,
        hypothesis.projectId,
        definition.lifecycleStatus,
      );
    }
  }
  for (const observation of overview.observations) {
    register("research_observation", observation.id, `Observation ${observation.id}`, projectId, observation.qualityStatus);
  }
  for (const design of overview.designVersions) {
    register("research_design", design.id, design.title, projectId, design.lifecycleStatus);
  }
  for (const pattern of overview.patterns) {
    register("research_pattern", pattern.id, pattern.title, projectId, pattern.lifecycleStatus);
  }
  for (const review of overview.literatureReviews) {
    register("research_literature_review", review.id, review.title, projectId, review.lifecycleStatus);
  }
  for (const adoption of overview.adoptions) {
    register(
      "framework_adoption",
      adoption.id,
      `${adoption.title} v${adoption.versionNo}`,
      projectId,
      null,
    );
    register(
      "framework_version",
      adoption.frameworkVersionId,
      `${adoption.title} v${adoption.versionNo}`,
      null,
      "adopted; FrameworkVersion remains workspace-shared",
    );
  }

  for (const reference of overview.references) {
    if (reference.sourceClaimId) {
      register(
        "source_claim",
        reference.sourceClaimId,
        `SourceClaim ${reference.sourceClaimId}`,
        null,
        "attributed evidence",
        referenceHref("source_claim", reference.sourceClaimId, overview),
      );
    }
    if (reference.sourceLocationId) {
      register(
        "source_version_segment",
        reference.sourceLocationId,
        `SourceLocation ${reference.sourceLocationId}`,
        null,
        null,
        referenceHref("source_version_segment", reference.sourceLocationId, overview),
      );
    }
    if (reference.sourceVersionId) {
      register(
        "source_version",
        reference.sourceVersionId,
        `SourceVersion ${reference.sourceVersionId}`,
        null,
        null,
        referenceHref("source_version", reference.sourceVersionId, overview),
      );
    }
    if (reference.sourceLocationId) {
      register(
        "source_location",
        reference.sourceLocationId,
        `SourceLocation ${reference.sourceLocationId}`,
        null,
        null,
        referenceHref("source_location", reference.sourceLocationId, overview),
      );
    }
  }

  for (const hypothesis of overview.hypotheses) {
    const hypothesisRef = ref("research_hypothesis", hypothesis.id, `Hypothesis ${hypothesis.id}`, hypothesis.projectId);
    if (hypothesis.researchQuestionId) {
      connect(
        "Research",
        hypothesisRef,
        "addresses",
        ref("research_question", hypothesis.researchQuestionId, `Research question ${hypothesis.researchQuestionId}`, projectId),
        "research_hypothesis",
        hypothesis.id,
      );
    }
    for (const competingId of hypothesis.competingHypothesisIds) {
      connect(
        "Research",
        hypothesisRef,
        "competes with",
        ref("research_hypothesis", competingId, `Hypothesis ${competingId}`),
        "research_hypothesis",
        hypothesis.id,
      );
    }
    for (const evidence of hypothesis.evidence) {
      const targetKind = evidence.targetKind === "observation"
        ? "research_observation"
        : evidence.targetKind === "pattern"
          ? "research_pattern"
          : "source_claim";
      connect(
        "Research evidence",
        hypothesisRef,
        `${evidence.polarity} ${evidence.targetKind}`,
        ref(targetKind, evidence.targetId, `${evidence.targetKind} ${evidence.targetId}`),
        "research_hypothesis",
        hypothesis.id,
        evidence.rationale,
      );
      if (evidence.sourceVersionId) {
        connect(
          "Research provenance",
          ref(targetKind, evidence.targetId, `${evidence.targetKind} ${evidence.targetId}`),
          "pins exact SourceVersion",
          ref("source_version", evidence.sourceVersionId, `SourceVersion ${evidence.sourceVersionId}`),
          "research_hypothesis",
          hypothesis.id,
        );
      }
      if (evidence.sourceLocationId) {
        connect(
          "Research provenance",
          ref(targetKind, evidence.targetId, `${evidence.targetKind} ${evidence.targetId}`),
          "pins exact SourceLocation",
          ref("source_location", evidence.sourceLocationId, `SourceLocation ${evidence.sourceLocationId}`),
          "research_hypothesis",
          hypothesis.id,
        );
      }
    }
    for (const modelReference of hypothesis.modelReferences) {
      connect(
        "Research → Model",
        hypothesisRef,
        "has explicit unresolved reference",
        ref("model_version", modelReference.modelVersionId, `ModelVersion ${modelReference.modelVersionId}`),
        "hypothesis_model_reference",
        modelReference.id,
        modelReference.resolutionStatus,
      );
    }
  }
  for (const reference of overview.references) {
    const recordKind = reference.kind === "literature_review"
      ? "research_literature_review"
      : "research_hypothesis";
    const sourceRecord = ref(
      recordKind,
      reference.recordId,
      `${reference.kind === "literature_review" ? "Literature review" : "Hypothesis"} ${reference.recordTitle}`,
      projectId,
    );
    if (reference.sourceClaimId) {
      connect(
        "Research citation",
        sourceRecord,
        "cites exact SourceClaim",
        ref("source_claim", reference.sourceClaimId, `SourceClaim ${reference.sourceClaimId}`),
        reference.kind,
        reference.recordId,
      );
    }
    if (reference.sourceVersionId) {
      connect(
        "Research citation",
        sourceRecord,
        "pins exact SourceVersion",
        ref("source_version", reference.sourceVersionId, `SourceVersion ${reference.sourceVersionId}`),
        reference.kind,
        reference.recordId,
      );
    }
    if (reference.sourceLocationId) {
      connect(
        "Research citation",
        sourceRecord,
        "pins exact SourceLocation",
        ref("source_location", reference.sourceLocationId, `SourceLocation ${reference.sourceLocationId}`),
        reference.kind,
        reference.recordId,
      );
    }
  }
  for (const adoption of overview.adoptions) {
    connect(
      "Framework adoption",
      ref("framework_adoption", adoption.id, `Project adoption ${adoption.id}`, projectId),
      "references shared FrameworkVersion",
      ref("framework_version", adoption.frameworkVersionId, `FrameworkVersion ${adoption.frameworkVersionId}`),
      "project_knowledge_adoption",
      adoption.id,
      "Adoption does not transfer ownership.",
    );
  }

  const analysis = input.analysis.data;
  if (analysis) {
    for (const definition of analysis.definitions.filter((item) => item.projectId === projectId)) {
      register("analysis_definition", definition.id, definition.title, projectId, definition.lifecycleStatus);
    }
    for (const method of analysis.methods.filter((item) => item.projectId === projectId)) {
      register("analysis_method", method.id, method.name, projectId, method.verificationStatus);
    }
    for (const metric of analysis.metrics.filter((item) => item.projectId === projectId)) {
      register("analysis_metric", metric.id, metric.name, projectId, metric.verificationStatus);
    }
    for (const run of analysis.runs.filter((item) => item.projectId === projectId)) {
      register("analysis_run", run.id, `AnalysisRun ${run.id}`, projectId, run.lifecycleStatus);
      for (const input of run.inputs) {
        register("analysis_input", input.id, `Analysis input ${input.id}`, projectId, input.role);
      }
      for (const output of run.outputs) {
        register("analysis_output", output.id, output.title, projectId, output.outputKind);
      }
      for (const metric of run.metrics) {
        register("analysis_metric_result", metric.id, `Analysis metric ${metric.id}`, projectId, null);
      }
    }
    for (const evidence of analysis.evidence.filter((item) => item.projectId === projectId)) {
      register("analysis_evidence", evidence.id, `Evidence ${evidence.id}`, projectId, evidence.status);
    }
    for (const finding of analysis.findings.filter((item) => item.projectId === projectId)) {
      register("finding_version", finding.id, finding.title, projectId, finding.status);
    }
  }

  const experiment = input.experiment.data;
  if (experiment) {
    const projectDatasetVersionIds = new Set(
      experiment.datasets
        .filter((item) => item.projectId === projectId)
        .flatMap((dataset) => dataset.versions.map((version) => version.id)),
    );
    for (const design of experiment.designVersions.filter((item) => item.projectId === projectId)) {
      register("experiment_design", design.id, design.title, projectId, design.lifecycleStatus);
    }
    for (const run of experiment.runs.filter((item) => item.projectId === projectId)) {
      register("experiment_run", run.id, `ExperimentRun ${run.id}`, projectId, run.lifecycleStatus);
    }
    for (const dataset of experiment.datasets.filter((item) => item.projectId === projectId)) {
      for (const version of dataset.versions) {
        register(
          "dataset_version",
          version.id,
          `${dataset.title} v${version.versionNo}`,
          projectId,
          version.lifecycleStatus,
        );
      }
    }
    for (const transformation of experiment.transformations.filter(
      (item) => projectDatasetVersionIds.has(item.outputDatasetVersionId),
    )) {
      register("dataset_transformation", transformation.id, `Transformation ${transformation.method}`, projectId, null);
    }
  }

  const model = input.model.data;
  if (model) {
    for (const version of model.versions.filter((item) => item.createdInProjectId === projectId)) {
      const modelTitle = model.models.find((item) => item.id === version.modelId)?.displayName ?? version.modelId;
      register(
        "model_version",
        version.id,
        `${modelTitle} v${version.versionNo}`,
        version.createdInProjectId,
        version.lifecycleStatus,
      );
    }
    for (const evidence of model.evidence.filter((item) => item.projectId === projectId)) {
      register("model_evidence", evidence.id, `ModelEvidence ${evidence.id}`, projectId, evidence.polarity);
    }
    for (const evaluation of model.evaluations.filter((item) => item.projectId === projectId)) {
      register("model_evaluation", evaluation.id, `ModelEvaluation ${evaluation.id}`, projectId, null);
    }
    for (const validation of model.validations.filter((item) => item.projectId === projectId)) {
      register("model_validation", validation.id, `ModelValidation ${validation.id}`, projectId, validation.outcome);
    }
    for (const replication of model.replications.filter((item) => item.projectId === projectId)) {
      register("model_replication", replication.id, `ModelReplication ${replication.id}`, projectId, replication.outcome);
    }
  }

  const transfer = input.transfer.data;
  if (transfer) {
    for (const artifact of transfer.artifacts) {
      const kind = transferArtifactKind(artifact.artifact);
      const id = transferArtifactId(artifact.artifact);
      register(kind, id, `${artifact.title} v${artifact.versionNo}`, artifact.ownerProjectId, artifact.lifecycleStatus);
    }
    for (const knowledge of transfer.knowledgeVersions) {
      register(
        `${knowledge.kind}_version`,
        knowledge.versionId,
        `${knowledge.title} v${knowledge.versionNo}`,
        knowledge.projectId,
        knowledge.status,
      );
    }
    const applicableTransfers = transfer.transfers.filter(
      (item) => item.origin.originProjectId === projectId || item.receivingProjectId === projectId,
    );
    const applicableTransferIds = new Set(applicableTransfers.map((item) => item.id));
    const applicableOrigins = transfer.origins.filter(
      (item) => item.originProjectId === projectId
        || applicableTransfers.some((candidate) => candidate.origin.id === item.id),
    );
    const applicableEligibilities = transfer.eligibilities.filter(
      (item) => item.origin.originProjectId === projectId
        || applicableTransfers.some((candidate) => candidate.eligibilityId === item.id),
    );

    for (const origin of applicableOrigins) {
      register("transfer_origin", origin.id, `Origin ${origin.artifact.kind}`, origin.originProjectId, null);
    }
    for (const eligibility of applicableEligibilities) {
      register("transfer_eligibility", eligibility.id, `Eligibility ${eligibility.id}`, eligibility.origin.originProjectId, eligibility.state);
    }
    for (const item of applicableTransfers) {
      register("knowledge_transfer", item.id, `Transfer ${item.id}`, null, item.state);
    }
    for (const adaptation of transfer.adaptations.filter(
      (item) => item.receivingProjectId === projectId || applicableTransferIds.has(item.transferId),
    )) {
      register("transfer_adaptation", adaptation.id, adaptation.title, adaptation.receivingProjectId, null);
    }
    for (const usage of transfer.downstreamUsage.filter(
      (item) => item.projectId === projectId || applicableTransferIds.has(item.transferId),
    )) {
      register("downstream_usage", usage.id, `Downstream use ${usage.id}`, usage.projectId, null);
    }
    for (const lineage of transfer.lineage.filter(
      (item) => item.originProjectId === projectId || item.receivingProjectId === projectId,
    )) {
      register("transfer_lineage", lineage.id, `Lineage ${lineage.id}`, null, lineage.relationKind);
    }

    for (const origin of applicableOrigins) {
      const artifact = ref(
        transferArtifactKind(origin.artifact),
        transferArtifactId(origin.artifact),
        `Origin artifact ${transferArtifactId(origin.artifact)}`,
        origin.originProjectId,
      );
      connect(
        "Knowledge Transfer",
        ref("transfer_origin", origin.id, `Origin ${origin.id}`, origin.originProjectId),
        "records exact origin artifact",
        artifact,
        "transfer_origin",
        origin.id,
      );
    }
    for (const eligibility of applicableEligibilities) {
      connect(
        "Knowledge Transfer",
        ref("transfer_eligibility", eligibility.id, `Eligibility ${eligibility.id}`),
        "governs",
        ref("transfer_origin", eligibility.origin.id, `Origin ${eligibility.origin.id}`, eligibility.origin.originProjectId),
        "transfer_eligibility",
        eligibility.id,
        eligibility.state,
      );
    }
    for (const item of applicableTransfers) {
      connect(
        "Knowledge Transfer",
        ref("knowledge_transfer", item.id, `Transfer ${item.id}`, item.receivingProjectId),
        "uses eligibility",
        ref("transfer_eligibility", item.eligibilityId, `Eligibility ${item.eligibilityId}`),
        "knowledge_transfer",
        item.id,
        `${item.origin.originProjectTitle} → ${item.receivingProjectTitle} · ${item.state}`,
      );
      connect(
        "Knowledge Transfer",
        ref("knowledge_transfer", item.id, `Transfer ${item.id}`, item.receivingProjectId),
        "transfers exact origin",
        ref(
          transferArtifactKind(item.origin.artifact),
          transferArtifactId(item.origin.artifact),
          `Origin artifact ${transferArtifactId(item.origin.artifact)}`,
          item.origin.originProjectId,
        ),
        "knowledge_transfer",
        item.id,
      );
    }
    for (const adaptation of transfer.adaptations.filter(
      (item) => item.receivingProjectId === projectId || applicableTransferIds.has(item.transferId),
    )) {
      connect(
        "Knowledge Transfer",
        ref("transfer_adaptation", adaptation.id, adaptation.title, adaptation.receivingProjectId),
        "adapts",
        ref(
          transferArtifactKind(adaptation.origin.artifact),
          transferArtifactId(adaptation.origin.artifact),
          `Origin artifact ${transferArtifactId(adaptation.origin.artifact)}`,
          adaptation.origin.originProjectId,
        ),
        "transfer_adaptation",
        adaptation.id,
        "Adaptation is a separate artifact; it does not change origin ownership.",
      );
      connect(
        "Knowledge Transfer",
        ref("transfer_adaptation", adaptation.id, adaptation.title, adaptation.receivingProjectId),
        "is recorded by",
        ref("knowledge_transfer", adaptation.transferId, `Transfer ${adaptation.transferId}`),
        "transfer_adaptation",
        adaptation.id,
      );
    }
    for (const lineage of transfer.lineage.filter(
      (item) => item.originProjectId === projectId || item.receivingProjectId === projectId,
    )) {
      connect(
        "Knowledge Transfer",
        ref("transfer_lineage", lineage.id, `Lineage ${lineage.id}`, lineage.receivingProjectId),
        lineage.relationKind,
        ref(
          transferArtifactKind(lineage.origin),
          transferArtifactId(lineage.origin),
          `Origin artifact ${transferArtifactId(lineage.origin)}`,
          lineage.originProjectId,
        ),
        "transfer_lineage",
        lineage.id,
      );
      if (lineage.adaptation) {
        connect(
          "Knowledge Transfer",
          ref("transfer_lineage", lineage.id, `Lineage ${lineage.id}`, lineage.receivingProjectId),
          "records adaptation",
          ref(
            transferArtifactKind(lineage.adaptation),
            transferArtifactId(lineage.adaptation),
            `Adaptation artifact ${transferArtifactId(lineage.adaptation)}`,
            lineage.receivingProjectId,
          ),
          "transfer_lineage",
          lineage.id,
        );
      }
    }
    for (const usage of transfer.downstreamUsage.filter(
      (item) => item.projectId === projectId || applicableTransferIds.has(item.transferId),
    )) {
      const targetKind = usage.reference.kind === "model_evaluation"
        ? "model_evaluation"
        : usage.reference.kind === "analysis_run"
          ? "analysis_run"
          : usage.reference.kind === "experiment_run"
            ? "experiment_run"
            : `${usage.reference.kind.replace("_version", "")}_version`;
      const targetId = "id" in usage.reference ? usage.reference.id : usage.reference.versionId;
      connect(
        "Knowledge Transfer",
        ref("downstream_usage", usage.id, `Downstream use ${usage.id}`, usage.projectId),
        "records use of",
        ref(targetKind, targetId, `${usage.reference.kind} ${targetId}`, usage.projectId),
        "downstream_usage",
        usage.id,
        usage.useContext,
      );
      if (applicableTransferIds.has(usage.transferId)) {
        connect(
          "Knowledge Transfer",
          ref("downstream_usage", usage.id, `Downstream use ${usage.id}`, usage.projectId),
          "under transfer",
          ref("knowledge_transfer", usage.transferId, `Transfer ${usage.transferId}`),
          "downstream_usage",
          usage.id,
        );
      }
    }

  }

  for (const design of input.experiment.data?.designVersions.filter((item) => item.projectId === projectId) ?? []) {
    const designRef = ref("experiment_design", design.id, design.title, projectId);
    const specification = design.specification;
    if (!specification) continue;
    if (specification.targetResearchQuestionId) {
      connect(
        "Research → Experiment",
        ref("research_question", specification.targetResearchQuestionId, `Research question ${specification.targetResearchQuestionId}`),
        "targets",
        designRef,
        "experiment_design",
        design.id,
      );
    }
    if (specification.targetHypothesisVersionId) {
      connect(
        "Research → Experiment",
        ref("hypothesis_version", specification.targetHypothesisVersionId, `HypothesisVersion ${specification.targetHypothesisVersionId}`),
        "tests",
        designRef,
        "experiment_design",
        design.id,
      );
    }
    if (specification.targetModelVersionId) {
      connect(
        "Experiment → Model",
        ref("model_version", specification.targetModelVersionId, `ModelVersion ${specification.targetModelVersionId}`),
        "targets (recorded ID; not ownership validation)",
        designRef,
        "experiment_design",
        design.id,
      );
    }
    for (const pin of specification.datasets) {
      connect(
        "Experiment → Dataset",
        designRef,
        `pins ${pin.role}`,
        ref("dataset_version", pin.datasetVersionId, `DatasetVersion ${pin.datasetVersionId}`, projectId),
        "experiment_design",
        design.id,
      );
    }
  }

  for (const run of input.experiment.data?.runs.filter((item) => item.projectId === projectId) ?? []) {
    connect(
      "Experiment",
      ref("experiment_run", run.id, `ExperimentRun ${run.id}`, projectId),
      "executes",
      ref("experiment_design", run.designVersionId, `ExperimentDesignVersion ${run.designVersionId}`, projectId),
      "experiment_run",
      run.id,
    );
    for (const pin of run.datasetPins) {
      connect(
        "Experiment → Dataset",
        ref("experiment_run", run.id, `ExperimentRun ${run.id}`, projectId),
        `${pin.usageKind} ${pin.role}`,
        ref("dataset_version", pin.datasetVersionId, `DatasetVersion ${pin.datasetVersionId}`, projectId),
        "experiment_run",
        run.id,
      );
    }
    const projectDatasetVersionIds = new Set(
      (input.experiment.data?.datasets ?? [])
        .filter((item) => item.projectId === projectId)
        .flatMap((dataset) => dataset.versions.map((version) => version.id)),
    );
    for (const transformation of (input.experiment.data?.transformations ?? []).filter(
      (item) => projectDatasetVersionIds.has(item.outputDatasetVersionId),
    )) {
      connect(
        "Dataset lineage",
        ref("dataset_transformation", transformation.id, `Transformation ${transformation.id}`, projectId),
        "produces",
        ref("dataset_version", transformation.outputDatasetVersionId, `DatasetVersion ${transformation.outputDatasetVersionId}`, projectId),
        "dataset_transformation",
        transformation.id,
        transformation.method,
      );
      for (const item of transformation.inputs) {
        connect(
          "Dataset lineage",
          ref("dataset_transformation", transformation.id, `Transformation ${transformation.id}`, projectId),
          `uses ${item.role}`,
          ref("dataset_version", item.datasetVersionId, `DatasetVersion ${item.datasetVersionId}`, projectId),
          "dataset_transformation",
          transformation.id,
        );
      }
    }
  }

  for (const run of analysis?.runs.filter((item) => item.projectId === projectId) ?? []) {
    connect(
      "Analysis",
      ref("analysis_run", run.id, `AnalysisRun ${run.id}`, projectId),
      "uses exact definition",
      ref("analysis_definition", run.definitionVersionId, `AnalysisDefinitionVersion ${run.definitionVersionId}`, projectId),
      "analysis_run",
      run.id,
    );
    for (const inputPin of run.inputs) {
      connect(
        "Analysis → Dataset",
        ref("analysis_input", inputPin.id, `Analysis input ${inputPin.id}`, projectId),
        `uses ${inputPin.role}`,
        ref("dataset_version", inputPin.datasetVersionId, `DatasetVersion ${inputPin.datasetVersionId}`, projectId),
        "analysis_run",
        run.id,
      );
    }
    for (const output of run.outputs) {
      connect(
        "Analysis",
        ref("analysis_run", run.id, `AnalysisRun ${run.id}`, projectId),
        "produces",
        ref("analysis_output", output.id, output.title, projectId),
        "analysis_run",
        run.id,
      );
    }
    for (const metric of run.metrics) {
      connect(
        "Analysis",
        ref("analysis_run", run.id, `AnalysisRun ${run.id}`, projectId),
        "uses exact method",
        ref("analysis_method", metric.methodVersionId, `AnalysisMethodVersion ${metric.methodVersionId}`, projectId),
        "analysis_run",
        run.id,
        `Metric result ${metric.id}`,
      );
      connect(
        "Analysis",
        ref("analysis_run", run.id, `AnalysisRun ${run.id}`, projectId),
        "uses exact metric",
        ref("analysis_metric", metric.metricDefinitionVersionId, `MetricDefinitionVersion ${metric.metricDefinitionVersionId}`, projectId),
        "analysis_run",
        run.id,
        `Metric result ${metric.id}`,
      );
    }
  }

  for (const definition of analysis?.definitions.filter((item) => item.projectId === projectId) ?? []) {
    connect(
      "Analysis definition",
      ref("analysis_definition", definition.id, definition.title, projectId),
      "pins method",
      ref("analysis_method", definition.methodVersionId, `AnalysisMethodVersion ${definition.methodVersionId}`, projectId),
      "analysis_definition",
      definition.id,
    );
    for (const metricId of definition.metricDefinitionVersionIds) {
      connect(
        "Analysis definition",
        ref("analysis_definition", definition.id, definition.title, projectId),
        "pins metric",
        ref("analysis_metric", metricId, `MetricDefinitionVersion ${metricId}`, projectId),
        "analysis_definition",
        definition.id,
      );
    }
  }

  for (const evidence of analysis?.evidence.filter((item) => item.projectId === projectId) ?? []) {
    const proposition = evidence.proposition;
    connect(
      "Analysis evidence",
      ref("analysis_evidence", evidence.id, `Evidence ${evidence.id}`, projectId),
      `attributes ${evidence.polarity} evidence to`,
      ref(proposition.kind, proposition.id, `${proposition.kind} ${proposition.id}`),
      "analysis_evidence",
      evidence.id,
      evidence.qualityRationale,
    );
    connect(
      "Analysis evidence",
      ref("analysis_evidence", evidence.id, `Evidence ${evidence.id}`, projectId),
      "cites artifact",
      ref(evidence.artifact.kind, evidence.artifact.id, `${evidence.artifact.kind} ${evidence.artifact.id}`),
      "analysis_evidence",
      evidence.id,
      evidence.limitations || null,
    );
  }

  for (const finding of analysis?.findings.filter((item) => item.projectId === projectId) ?? []) {
    connect(
      "Analysis finding",
      ref("finding_version", finding.id, finding.title, projectId),
      "asserts proposition",
      ref(finding.proposition.kind, finding.proposition.id, `${finding.proposition.kind} ${finding.proposition.id}`),
      "finding_version",
      finding.id,
    );
    if (finding.parentVersionId) {
      connect(
        "Analysis finding lineage",
        ref("finding_version", finding.id, finding.title, projectId),
        "revises",
        ref("finding_version", finding.parentVersionId, `FindingVersion ${finding.parentVersionId}`),
        "finding_version",
        finding.id,
        finding.changeRationale,
      );
    }
    for (const runId of finding.analysisRunIds) {
      connect(
        "Analysis finding",
        ref("finding_version", finding.id, finding.title, projectId),
        "synthesizes",
        ref("analysis_run", runId, `AnalysisRun ${runId}`, projectId),
        "finding_version",
        finding.id,
      );
    }
    for (const link of finding.evidence) {
      connect(
        "Analysis finding",
        ref("finding_version", finding.id, finding.title, projectId),
        `uses ${link.synthesisPolarity} evidence`,
        ref("analysis_evidence", link.evidenceId, `Evidence ${link.evidenceId}`, projectId),
        "finding_version",
        finding.id,
        link.rationale,
      );
    }
    for (const relation of finding.relations) {
      connect(
        "Analysis finding relation",
        ref("finding_version", finding.id, finding.title, projectId),
        relation.relationKind,
        ref("finding_version", relation.targetVersionId, `FindingVersion ${relation.targetVersionId}`),
        "finding_relation",
        relation.id,
        relation.rationale,
      );
    }
  }

  for (const version of model?.versions.filter((item) => item.createdInProjectId === projectId) ?? []) {
    const versionRef = ref("model_version", version.id, `ModelVersion ${version.id}`, projectId);
    for (const parent of version.parents) {
      connect(
        "Model lineage",
        versionRef,
        "derives from",
        ref("model_version", parent.versionId, `ModelVersion ${parent.versionId}`),
        "model_version",
        version.id,
        parent.rationale,
      );
    }
    for (const hypothesis of version.hypotheses) {
      connect(
        "Model → Research",
        versionRef,
        hypothesis.relationKind,
        ref("hypothesis_version", hypothesis.targetId, `HypothesisVersion ${hypothesis.targetId}`),
        "model_version",
        version.id,
        hypothesis.rationale,
      );
    }
    for (const finding of version.findings) {
      connect(
        "Model → Analysis",
        versionRef,
        finding.relationKind,
        ref("finding_version", finding.targetId, `FindingVersion ${finding.targetId}`),
        "model_version",
        version.id,
        finding.rationale,
      );
    }
    for (const experimentLink of version.experiments) {
      connect(
        "Model → Experiment",
        versionRef,
        experimentLink.relationKind,
        ref("experiment_design", experimentLink.designVersionId, `ExperimentDesignVersion ${experimentLink.designVersionId}`),
        "model_version",
        version.id,
        experimentLink.rationale,
      );
      if (experimentLink.experimentRunId) {
        connect(
          "Model → Experiment",
          versionRef,
          "links run",
          ref("experiment_run", experimentLink.experimentRunId, `ExperimentRun ${experimentLink.experimentRunId}`),
          "model_version",
          version.id,
        );
      }
    }
    for (const dataset of version.datasetUses) {
      connect(
        "Model → Dataset",
        versionRef,
        `uses ${dataset.role}`,
        ref("dataset_version", dataset.datasetVersionId, `DatasetVersion ${dataset.datasetVersionId}`),
        "model_version",
        version.id,
        dataset.rationale,
      );
    }
    for (const impact of version.impacts) {
      const target = impact.target;
      if (target.kind === "assumption") continue;
      const targetKind = target.kind === "source_claim"
        ? "source_claim"
        : target.kind;
      const targetId = target.kind === "source_claim"
        ? target.claimId
        : target.kind === "hypothesis_version"
          ? target.hypothesisVersionId
          : target.findingVersionId;
      connect(
        "Model impact",
        versionRef,
        `impacts (${impact.impactKind})`,
        ref(targetKind, targetId, `${targetKind} ${targetId}`),
        "model_version",
        version.id,
        impact.rationale,
      );
    }
    for (const contribution of version.contributions) {
      const target = contribution.target;
      if (target.kind === "hypothesis_version" || target.kind === "finding_version") {
        connect(
          "Model contribution",
          versionRef,
          contribution.relationKind,
          ref(target.kind, target.id, `${target.kind} ${target.id}`),
          "model_contribution",
          contribution.id,
          contribution.rationale,
        );
      } else if (target.kind === "experiment") {
        connect(
          "Model contribution",
          versionRef,
          contribution.relationKind,
          ref("experiment_design", target.designVersionId, `ExperimentDesignVersion ${target.designVersionId}`),
          "model_contribution",
          contribution.id,
          contribution.rationale,
        );
        if (target.experimentRunId) {
          connect(
            "Model contribution",
            versionRef,
            "links exact ExperimentRun",
            ref("experiment_run", target.experimentRunId, `ExperimentRun ${target.experimentRunId}`),
            "model_contribution",
            contribution.id,
            contribution.rationale,
          );
        }
      } else if (target.kind === "dataset") {
        connect(
          "Model contribution",
          versionRef,
          `uses ${target.role}`,
          ref("dataset_version", target.datasetVersionId, `DatasetVersion ${target.datasetVersionId}`),
          "model_contribution",
          contribution.id,
          contribution.rationale,
        );
      } else {
        connect(
          "Model contribution",
          versionRef,
          "cites evidence",
          ref("model_evidence", target.id, `ModelEvidence ${target.id}`, projectId),
          "model_contribution",
          contribution.id,
          contribution.rationale,
        );
      }
    }
    for (const evidenceId of version.evidenceIds) {
      connect(
        "Model evidence",
        versionRef,
        "cites",
        ref("model_evidence", evidenceId, `ModelEvidence ${evidenceId}`, projectId),
        "model_version",
        version.id,
      );
    }
  }

  for (const evidence of model?.evidence.filter((item) => item.projectId === projectId) ?? []) {
    connect(
      "Model evidence",
      ref("model_evidence", evidence.id, `ModelEvidence ${evidence.id}`, projectId),
      "supports model version",
      ref("model_version", evidence.modelVersionId, `ModelVersion ${evidence.modelVersionId}`, projectId),
      "model_evidence",
      evidence.id,
    );
    connect(
      "Model evidence",
      ref("model_evidence", evidence.id, `ModelEvidence ${evidence.id}`, projectId),
      "cites artifact",
      ref(evidence.artifact.kind, evidence.artifact.id, `${evidence.artifact.kind} ${evidence.artifact.id}`),
      "model_evidence",
      evidence.id,
      evidence.limitations || null,
    );
  }
  for (const evaluation of model?.evaluations.filter((item) => item.projectId === projectId) ?? []) {
    const evaluationRef = ref("model_evaluation", evaluation.id, `ModelEvaluation ${evaluation.id}`, projectId);
    connect("Model evaluation", evaluationRef, "evaluates", ref("model_version", evaluation.modelVersionId, `ModelVersion ${evaluation.modelVersionId}`), "model_evaluation", evaluation.id);
    connect("Model evaluation", evaluationRef, "uses AnalysisRun", ref("analysis_run", evaluation.analysisRunId, `AnalysisRun ${evaluation.analysisRunId}`), "model_evaluation", evaluation.id);
    connect("Model evaluation", evaluationRef, "uses exact run input", ref("analysis_input", evaluation.analysisRunInputId, `Analysis input ${evaluation.analysisRunInputId}`), "model_evaluation", evaluation.id);
    connect("Model evaluation", evaluationRef, "pins DatasetVersion", ref("dataset_version", evaluation.datasetVersionId, `DatasetVersion ${evaluation.datasetVersionId}`), "model_evaluation", evaluation.id);
    connect("Model evaluation", evaluationRef, "uses AnalysisMetric", ref("analysis_metric_result", evaluation.analysisMetricId, `AnalysisMetric ${evaluation.analysisMetricId}`), "model_evaluation", evaluation.id, `MetricDefinitionVersion ${evaluation.metricDefinitionVersionId}`);
    connect("Model evaluation", evaluationRef, "pins MetricDefinitionVersion", ref("analysis_metric", evaluation.metricDefinitionVersionId, `MetricDefinitionVersion ${evaluation.metricDefinitionVersionId}`), "model_evaluation", evaluation.id);
  }
  for (const validation of model?.validations.filter((item) => item.projectId === projectId) ?? []) {
    const targetKind = validation.target.kind === "model_version"
      ? "model_version"
      : validation.target.kind;
    const targetId = validation.target.id;
    connect(
      "Model validation",
      ref("model_validation", validation.id, `ModelValidation ${validation.id}`, projectId),
      "validates",
      ref(targetKind, targetId, `${targetKind} ${targetId}`),
      "model_validation",
      validation.id,
      validation.outcome ?? "not finalized",
    );
    for (const evidenceId of validation.analysisEvidenceIds) {
      connect("Model validation", ref("model_validation", validation.id, `ModelValidation ${validation.id}`, projectId), "uses AnalysisEvidence", ref("analysis_evidence", evidenceId, `Evidence ${evidenceId}`, projectId), "model_validation", validation.id);
    }
    for (const evidenceId of validation.modelEvidenceIds) {
      connect("Model validation", ref("model_validation", validation.id, `ModelValidation ${validation.id}`, projectId), "uses ModelEvidence", ref("model_evidence", evidenceId, `ModelEvidence ${evidenceId}`, projectId), "model_validation", validation.id);
    }
  }
  for (const replication of model?.replications.filter((item) => item.projectId === projectId) ?? []) {
    const replicationRef = ref("model_replication", replication.id, `ModelReplication ${replication.id}`, projectId);
    connect("Model replication", replicationRef, "replicates", ref("model_version", replication.modelVersionId, `ModelVersion ${replication.modelVersionId}`), "model_replication", replication.id);
    connect("Model replication", replicationRef, "uses original ExperimentRun", ref("experiment_run", replication.originalExperimentRunId, `ExperimentRun ${replication.originalExperimentRunId}`), "model_replication", replication.id);
    connect("Model replication", replicationRef, "uses original design", ref("experiment_design", replication.originalDesignVersionId, `ExperimentDesignVersion ${replication.originalDesignVersionId}`), "model_replication", replication.id);
    for (const dataset of replication.datasets) {
      connect("Model replication", replicationRef, `pins ${dataset.role}`, ref("dataset_version", dataset.datasetVersionId, `DatasetVersion ${dataset.datasetVersionId}`), "model_replication", replication.id);
    }
    for (const evidenceId of replication.analysisEvidenceIds) {
      connect("Model replication", replicationRef, "uses AnalysisEvidence", ref("analysis_evidence", evidenceId, `Evidence ${evidenceId}`), "model_replication", replication.id);
    }
    for (const evidenceId of replication.modelEvidenceIds) {
      connect("Model replication", replicationRef, "uses ModelEvidence", ref("model_evidence", evidenceId, `ModelEvidence ${evidenceId}`), "model_replication", replication.id);
    }
  }

  return connections;
}
