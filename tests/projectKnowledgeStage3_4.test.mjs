import assert from "node:assert/strict";
import test from "node:test";
import { buildProjectLifecycleConnections } from "../src/shared/layout/projectLifecycleModel.ts";

const projectA = "project-a";
const projectB = "project-b";

function emptyInput(projectId) {
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
      gaps: [],
      ideas: [],
      sources: [],
      availableFrameworkVersions: [],
      questionCount: 0,
      gapCount: 0,
      observationCount: 0,
      patternCount: 0,
      ideaCount: 0,
      designVersionCount: 0,
    },
    analysis: {
      status: "loaded",
      data: {
        definitions: [],
        methods: [],
        metrics: [],
        runs: [],
        evidence: [],
        findings: [],
      },
      error: null,
    },
    experiment: {
      status: "loaded",
      data: {
        designVersions: [],
        runs: [],
        datasets: [],
        splitGroups: [],
        transformations: [],
      },
      error: null,
    },
    model: {
      status: "loaded",
      data: {
        models: [],
        versions: [],
        evidence: [],
        evaluations: [],
        validations: [],
        replications: [],
      },
      error: null,
    },
    transfer: {
      status: "loaded",
      data: {
        projects: [],
        artifacts: [],
        knowledgeVersions: [],
        origins: [],
        eligibilities: [],
        transfers: [],
        adaptations: [],
        downstreamUsage: [],
        lineage: [],
      },
      error: null,
    },
  };
}

test("Stage 3.4 synthetic project lifecycle preserves exact IDs and project boundaries", () => {
  const input = emptyInput(projectA);

  input.overview.questions = [{
    id: "question-a",
    projectId: projectA,
    title: "Question A",
    lifecycleStatus: "active",
  }];

  input.overview.hypotheses = [{
    id: "hypothesis-a",
    projectId: projectA,
    researchQuestionId: "question-a",
    competingHypothesisIds: [],
    modelReferences: [],
    evidence: [
      {
        id: "evidence-a",
        targetKind: "source_claim",
        targetId: "claim-a",
        sourceId: "source-a",
        sourceVersionId: "source-version-a",
        sourceLocationId: "location-a",
        polarity: "supports",
        rationale: "Explicit evidence",
      },
    ],
    lifecycleStatus: "active",
    definitions: [{
      id: "hypothesis-version-a",
      versionNo: 1,
      statementH1: "Recorded H1",
      lifecycleStatus: "preregistered",
    }],
  }];

  input.overview.references = [{
    kind: "hypothesis_evidence",
    recordId: "hypothesis-a",
    recordTitle: "Hypothesis A",
    recordStatus: "active",
    sourceId: "source-a",
    sourceVersionId: "source-version-a",
    sourceLocationId: "location-a",
    sourceClaimId: "claim-a",
    note: "Attribution preserved",
    polarity: "supports",
  }];

  input.overview.adoptions = [{
    id: "adoption-a",
    projectId: projectA,
    title: "Framework adoption",
    versionNo: 3,
    frameworkVersionId: "framework-version-a",
    lifecycleStatus: "active",
  }];

  input.experiment.data.designVersions = [{
    id: "design-a",
    projectId: projectA,
    title: "Design A",
    lifecycleStatus: "active",
    specification: {
      targetResearchQuestionId: "question-a",
      targetHypothesisVersionId: "hypothesis-version-a",
      targetModelVersionId: "model-requested-version",
      datasets: [{ datasetVersionId: "dataset-a", role: "training" }],
    },
  }, {
    id: "design-b",
    projectId: projectB,
    title: "Design B",
    lifecycleStatus: "active",
    specification: {
      targetResearchQuestionId: "question-b",
      targetHypothesisVersionId: null,
      targetModelVersionId: null,
      datasets: [],
    },
  }];

  input.experiment.data.datasets = [{
    id: "dataset-root-a",
    projectId: projectA,
    title: "Dataset A",
    versions: [{
      id: "dataset-a",
      versionNo: 2,
      lifecycleStatus: "sealed",
    }],
  }];

  input.model.data.models = [{
    id: "model-a",
    displayName: "Model A",
    projectId: projectA,
    lifecycleStatus: "active",
  }];

  input.model.data.versions = [{
    id: "model-version-a",
    modelId: "model-a",
    versionNo: 4,
    createdInProjectId: projectA,
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

  input.transfer.data = {
    projects: [
      { id: projectA, title: "Project A", lifecycleStatus: "active" },
      { id: projectB, title: "Project B", lifecycleStatus: "active" },
    ],
    artifacts: [{
      artifact: { kind: "model_version", modelId: "model-a", versionId: "origin-version" },
      title: "Origin model",
      versionNo: 1,
      ownerProjectId: projectA,
      lifecycleStatus: "released",
      originId: "origin-a",
      eligibilityIds: ["eligibility-a"],
    }, {
      artifact: { kind: "model_version", modelId: "model-a", versionId: "recipient-version" },
      title: "Adapted model",
      versionNo: 1,
      ownerProjectId: projectB,
      lifecycleStatus: "draft",
      originId: null,
      eligibilityIds: [],
    }],
    knowledgeVersions: [],
    origins: [{
      id: "origin-a",
      artifact: { kind: "model_version", modelId: "model-a", versionId: "origin-version" },
      originProjectId: projectA,
      originProjectTitle: "Project A",
    }],
    eligibilities: [{
      id: "eligibility-a",
      origin: {
        id: "origin-a",
        artifact: { kind: "model_version", modelId: "model-a", versionId: "origin-version" },
        originProjectId: projectA,
        originProjectTitle: "Project A",
      },
      state: "eligible",
    }],
    transfers: [{
      id: "transfer-a",
      eligibilityId: "eligibility-a",
      origin: {
        id: "origin-a",
        artifact: { kind: "model_version", modelId: "model-a", versionId: "origin-version" },
        originProjectId: projectA,
        originProjectTitle: "Project A",
      },
      receivingProjectId: projectB,
      receivingProjectTitle: "Project B",
      intendedPurpose: "research",
      attributionText: "Attribution",
      state: "accepted",
    }],
    adaptations: [{
      id: "adaptation-a",
      transferId: "transfer-a",
      origin: {
        id: "origin-a",
        artifact: { kind: "model_version", modelId: "model-a", versionId: "origin-version" },
        originProjectId: projectA,
        originProjectTitle: "Project A",
      },
      receivingProjectId: projectB,
      artifact: { kind: "model_version", modelId: "model-a", versionId: "recipient-version" },
      title: "Adaptation A",
      rationale: "Explicit adaptation",
    }],
    downstreamUsage: [],
    lineage: [],
  };

  const links = buildProjectLifecycleConnections(input);

  assert.ok(links.some((link) =>
    link.from.id === "question-a" && link.to.id === "design-a" && link.relationship === "targets"));
  assert.ok(links.some((link) =>
    link.from.id === "hypothesis-version-a" && link.to.id === "design-a" && link.relationship === "tests"));
  assert.ok(links.some((link) =>
    link.from.id === "design-a" && link.to.id === "dataset-a" && link.relationship === "pins training"));
  assert.ok(links.some((link) =>
    link.from.id === "adoption-a" && link.to.id === "framework-version-a" && link.relationship === "references shared FrameworkVersion"));
  assert.ok(links.some((link) =>
    link.from.id === "origin-a" && link.to.id === "origin-version" && link.relationship === "records exact origin artifact"));
  assert.ok(links.some((link) =>
    link.from.id === "adaptation-a" && link.to.id === "origin-version" && link.relationship === "adapts"));
  assert.ok(!links.some((link) => link.to.id === "design-b"));
  assert.ok(!links.some((link) => link.from.id === "design-b"));
});

test("Stage 3.4 synthetic validation keeps exact requested versions explicit when unavailable", () => {
  const input = emptyInput(projectA);
  input.experiment.data.designVersions = [{
    id: "design-a",
    projectId: projectA,
    title: "Exact version gate",
    lifecycleStatus: "active",
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
    createdInProjectId: projectA,
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

  const links = buildProjectLifecycleConnections(input);
  const recordedLink = links.find((link) => link.relationship === "targets (recorded ID; not ownership validation)");

  assert.equal(recordedLink?.from.id, "requested-model-version");
  assert.equal(recordedLink?.from.availability, "unavailable");
  assert.notEqual(recordedLink?.from.id, "different-model-version");
});

test("Stage 3.4 synthetic validation preserves provenance and exact SourceVersion lineage", () => {
  const input = emptyInput(projectA);
  input.overview.references = [{
    kind: "hypothesis_evidence",
    recordId: "hypothesis-a",
    recordTitle: "Hypothesis A",
    recordStatus: "active",
    sourceId: "source-a",
    sourceVersionId: "source-version-a",
    sourceLocationId: "location-a",
    sourceClaimId: "claim-a",
    note: "A source attribution",
    polarity: "supports",
  }];
  input.overview.hypotheses = [{
    id: "hypothesis-a",
    projectId: projectA,
    researchQuestionId: "question-a",
    competingHypothesisIds: [],
    modelReferences: [],
    evidence: [{
      id: "evidence-a",
      targetKind: "source_claim",
      targetId: "claim-a",
      sourceId: "source-a",
      sourceVersionId: "source-version-a",
      sourceLocationId: "location-a",
      polarity: "supports",
      rationale: "Evidence from source claim",
    }],
    lifecycleStatus: "active",
    definitions: [{
      id: "hypothesis-version-a",
      versionNo: 1,
      statementH1: "Hypothesis A",
      lifecycleStatus: "active",
    }],
  }];
  input.overview.questions = [{
    id: "question-a",
    projectId: projectA,
    title: "Question A",
    lifecycleStatus: "active",
  }];

  const links = buildProjectLifecycleConnections(input);
  const sourceClaimLink = links.find((link) => link.to.kind === "source_claim");
  const sourceVersionLink = links.find((link) => link.to.kind === "source_version");
  const sourceLocationLink = links.find((link) => link.to.kind === "source_location");

  assert.equal(sourceClaimLink?.to.id, "claim-a");
  assert.equal(sourceVersionLink?.to.id, "source-version-a");
  assert.equal(sourceLocationLink?.to.id, "location-a");
  assert.match(sourceClaimLink?.to.href ?? "", /sourceClaimId=claim-a/);
  assert.match(sourceVersionLink?.to.href ?? "", /sourceVersionId=source-version-a/);
  assert.match(sourceLocationLink?.to.href ?? "", /sourceLocationId=location-a/);
});
