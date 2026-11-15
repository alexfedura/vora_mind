import assert from "node:assert/strict";
import test from "node:test";
import { buildProjectLifecycleConnections } from "../src/shared/layout/projectLifecycleModel.ts";

const projectId = "project-a";

function emptyInput() {
  return {
    projectId,
    overview: {
      questions: [],
      hypotheses: [],
      observations: [],
      patterns: [],
      designVersions: [],
      literatureReviews: [],
      adoptions: [],
      references: [],
    },
    analysis: { status: "loaded", data: {
      definitions: [], methods: [], metrics: [], runs: [], evidence: [], findings: [],
    }, error: null },
    experiment: { status: "loaded", data: {
      designVersions: [], runs: [], datasets: [], splitGroups: [], transformations: [],
    }, error: null },
    model: { status: "loaded", data: {
      models: [], versions: [], evidence: [], evaluations: [], validations: [], replications: [],
    }, error: null },
    transfer: { status: "loaded", data: {
      projects: [], artifacts: [], knowledgeVersions: [], origins: [], eligibilities: [],
      transfers: [], adaptations: [], downstreamUsage: [], lineage: [],
    }, error: null },
  };
}

test("the lifecycle model keeps explicit links project-scoped and preserves exact dataset pins", () => {
  const input = emptyInput();
  input.overview.questions = [{
    id: "question-a",
    title: "Question A",
    lifecycleStatus: "active",
  }];
  input.overview.hypotheses = [{
    id: "hypothesis-a",
    projectId,
    researchQuestionId: "question-a",
    competingHypothesisIds: [],
    modelReferences: [],
    evidence: [],
    definitions: [{
      id: "hypothesis-version-a",
      versionNo: 2,
      statementH1: "Recorded H1",
      lifecycleStatus: "preregistered",
    }],
  }];
  input.experiment.data.designVersions = [{
    id: "design-a",
    projectId,
    title: "Design A",
    specification: {
      targetResearchQuestionId: "question-a",
      targetHypothesisVersionId: "hypothesis-version-a",
      targetModelVersionId: null,
      datasets: [{ datasetVersionId: "dataset-exact", role: "training" }],
    },
  }, {
    id: "design-b",
    projectId: "project-b",
    title: "Other project",
    specification: {
      targetResearchQuestionId: "question-b",
      targetHypothesisVersionId: null,
      targetModelVersionId: null,
      datasets: [],
    },
  }];
  input.experiment.data.datasets = [{
    id: "dataset-id",
    projectId,
    title: "Dataset",
    versions: [{ id: "dataset-exact", versionNo: 4, lifecycleStatus: "sealed" }],
  }];

  const links = buildProjectLifecycleConnections(input);

  assert.ok(links.some((link) =>
    link.from.id === "question-a"
    && link.to.id === "design-a"
    && link.relationship === "targets"));
  assert.ok(links.some((link) =>
    link.from.id === "hypothesis-version-a"
    && link.to.id === "design-a"
    && link.relationship === "tests"));
  assert.ok(links.some((link) =>
    link.from.id === "design-a"
    && link.to.id === "dataset-exact"
    && link.to.availability === "available"));
  assert.ok(!links.some((link) => link.recordId === "design-b"));
});

test("an unavailable exact ModelVersion is not replaced by another local version", () => {
  const input = emptyInput();
  input.experiment.data.designVersions = [{
    id: "design-a",
    projectId,
    title: "Design A",
    specification: {
      targetResearchQuestionId: null,
      targetHypothesisVersionId: null,
      targetModelVersionId: "requested-model-version",
      datasets: [],
    },
  }];
  input.model.data.versions = [{
    id: "different-model-version",
    modelId: "model-a",
    versionNo: 9,
    createdInProjectId: projectId,
    lifecycleStatus: "released",
    parents: [],
    impacts: [],
    hypotheses: [],
    findings: [],
    experiments: [],
    datasetUses: [],
    evidenceIds: [],
    contributions: [],
  }];

  const link = buildProjectLifecycleConnections(input).find(
    (item) => item.relationship === "targets (recorded ID; not ownership validation)",
  );

  assert.equal(link?.from.id, "requested-model-version");
  assert.equal(link?.from.availability, "unavailable");
  assert.notEqual(link?.from.id, "different-model-version");
});

test("Analysis evidence keeps exact proposition and artifact provenance, including partial data", () => {
  const input = emptyInput();
  input.overview.references = [{
    kind: "hypothesis_evidence",
    recordId: "hypothesis-a",
    recordTitle: "Hypothesis A",
    recordStatus: "active",
    sourceId: "source-a",
    sourceVersionId: "source-version-a",
    sourceLocationId: "location-a",
    sourceClaimId: "claim-a",
    note: "Explicit citation",
    polarity: "supports",
  }];
  input.analysis.data.evidence = [{
    id: "evidence-a",
    projectId,
    proposition: { kind: "source_claim", id: "claim-a" },
    artifact: { kind: "source_version_segment", id: "location-a" },
    polarity: "supports",
    status: "assessed",
    qualityRationale: "Recorded rationale",
    limitations: "Recorded limits",
  }];

  const links = buildProjectLifecycleConnections(input);
  const proposition = links.find((link) => link.to.kind === "source_claim");
  const artifact = links.find((link) => link.to.kind === "source_version_segment");

  assert.equal(proposition?.to.id, "claim-a");
  assert.equal(proposition?.to.availability, "available");
  assert.match(proposition?.to.href ?? "", /sourceVersionId=source-version-a/);
  assert.equal(artifact?.to.id, "location-a");
  assert.match(artifact?.to.href ?? "", /sourceLocationId=location-a/);

  const empty = emptyInput();
  assert.deepEqual(buildProjectLifecycleConnections(empty), []);
});

test("Knowledge Transfer keeps origin and adapted versions distinct across project boundaries", () => {
  const input = emptyInput();
  const originArtifact = {
    kind: "model_version",
    modelId: "model-1",
    versionId: "origin-version",
  };
  const adaptationArtifact = {
    kind: "model_version",
    modelId: "model-1",
    versionId: "recipient-version",
  };
  const origin = {
    id: "origin-1",
    artifact: originArtifact,
    originProjectId: "origin-project",
    originProjectTitle: "Origin project",
  };
  input.transfer.data = {
    projects: [{ id: "origin-project", title: "Origin project", lifecycleStatus: "active" }],
    artifacts: [{
      artifact: originArtifact,
      title: "Model",
      versionNo: 3,
      ownerProjectId: "origin-project",
      lifecycleStatus: "released",
      originId: "origin-1",
      eligibilityIds: ["eligibility-1"],
    }, {
      artifact: adaptationArtifact,
      title: "Adapted model",
      versionNo: 1,
      ownerProjectId: projectId,
      lifecycleStatus: "draft",
      originId: null,
      eligibilityIds: [],
    }],
    knowledgeVersions: [],
    origins: [origin],
    eligibilities: [{
      id: "eligibility-1",
      origin,
      state: "eligible",
    }],
    transfers: [{
      id: "transfer-1",
      eligibilityId: "eligibility-1",
      origin,
      receivingProjectId: projectId,
      receivingProjectTitle: "Project A",
      intendedPurpose: "research",
      attributionText: "Attribution",
      state: "accepted",
    }],
    adaptations: [{
      id: "adaptation-1",
      transferId: "transfer-1",
      origin,
      receivingProjectId: projectId,
      artifact: adaptationArtifact,
      title: "Adaptation",
      rationale: "Explicit adaptation",
    }],
    downstreamUsage: [],
    lineage: [],
  };

  const links = buildProjectLifecycleConnections(input);
  const adaptationLink = links.find((link) =>
    link.from.id === "adaptation-1" && link.relationship === "adapts");

  assert.equal(adaptationLink?.to.id, "origin-version");
  assert.equal(adaptationLink?.to.availability, "outside_project");
  assert.equal(adaptationLink?.from.id, "adaptation-1");
  assert.notEqual(adaptationLink?.from.id, adaptationLink?.to.id);
  assert.match(adaptationLink?.note ?? "", /does not change origin ownership/);
});

test("FrameworkVersion adoption stays a project reference to a shared exact version", () => {
  const input = emptyInput();
  input.overview.adoptions = [{
    id: "adoption-1",
    frameworkId: "framework-1",
    frameworkVersionId: "framework-version-exact",
    title: "Framework",
    versionNo: 7,
  }];

  const link = buildProjectLifecycleConnections(input).find(
    (item) => item.recordId === "adoption-1" && item.relationship === "references shared FrameworkVersion",
  );

  assert.equal(link?.to.id, "framework-version-exact");
  assert.equal(link?.to.projectId, null);
  assert.match(link?.to.href ?? "", /framework-version-exact/);
  assert.match(link?.note ?? "", /does not transfer ownership/);
});
