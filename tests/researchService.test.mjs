import assert from "node:assert/strict";
import test, { mock } from "node:test";

const calls = [];
let response;
let failure;

mock.module("@tauri-apps/api/core", {
  exports: {
    invoke: async (command, args) => {
      calls.push({ command, args });
      if (failure) throw failure;
      return response;
    },
  },
});

const { researchService } = await import("../src/mind/services/researchService.ts");

test("project-context service passes the selected project and preserves exact citation IDs", async () => {
  calls.length = 0;
  failure = null;
  response = {
    projectId: "project-a",
    literatureReviews: [{
      projectId: "project-a",
      sources: [{
        sourceId: "shared-source",
        sourceVersionId: "source-version-a",
        sourceLocationId: "source-location-a",
        sourceClaimId: "source-claim-a",
      }],
    }],
  };

  const result = await researchService.getProjectContext("project-a");

  assert.deepEqual(calls, [{
    command: "research_get_project_context",
    args: { projectId: "project-a" },
  }]);
  assert.strictEqual(result, response);
  assert.equal(result.literatureReviews[0].sources[0].sourceVersionId, "source-version-a");
  assert.equal(result.literatureReviews[0].sources[0].sourceClaimId, "source-claim-a");
});

test("project-context service rejects a response for a different project", async () => {
  calls.length = 0;
  failure = null;
  response = { projectId: "project-a" };

  await assert.rejects(
    researchService.getProjectContext("project-b"),
    /did not match the requested project/,
  );
  assert.deepEqual(calls, [{
    command: "research_get_project_context",
    args: { projectId: "project-b" },
  }]);
});

test("project-context service rejects malformed empty and Discovery collection responses", async () => {
  calls.length = 0;
  failure = null;
  response = null;

  await assert.rejects(
    researchService.getProjectContext("project-empty"),
    /malformed/,
  );

  calls.length = 0;
  response = {
    projectId: "project-empty",
    phenomena: [],
    observationPlans: [],
    datasetVersions: [],
    observations: [],
    patterns: [],
    ideas: [],
    hypotheses: [],
    designVersions: [],
  };

  const result = await researchService.getProjectContext("project-empty");
  assert.deepEqual(result.phenomena, []);
  assert.deepEqual(result.observationPlans, []);
  assert.deepEqual(result.datasetVersions, []);
  assert.deepEqual(result.observations, []);
  assert.deepEqual(result.patterns, []);
  assert.deepEqual(result.ideas, []);
  assert.deepEqual(result.hypotheses, []);
  assert.deepEqual(result.designVersions, []);
});

test("project-context service preserves scoped first-principles discovery records", async () => {
  calls.length = 0;
  failure = null;
  response = {
    projectId: "project-discovery",
    phenomena: [{ id: "phenomenon-a", projectId: "project-discovery" }],
    observationPlans: [{ id: "plan-a", projectId: "project-discovery" }],
    observations: [{ id: "observation-a", projectId: "project-discovery" }],
    patterns: [{ id: "pattern-a", projectId: "project-discovery" }],
    ideas: [{ id: "idea-a", projectId: "project-discovery" }],
    hypotheses: [{ id: "hypothesis-a", projectId: "project-discovery" }],
    designVersions: [{ id: "design-a", projectId: "project-discovery" }],
  };

  const result = await researchService.getProjectContext("project-discovery");

  assert.deepEqual(calls, [{
    command: "research_get_project_context",
    args: { projectId: "project-discovery" },
  }]);
  assert.equal(result.phenomena[0].id, "phenomenon-a");
  assert.equal(result.observations[0].id, "observation-a");
  assert.equal(result.patterns[0].id, "pattern-a");
  assert.equal(result.hypotheses[0].id, "hypothesis-a");
  assert.equal(result.designVersions[0].id, "design-a");
});

test("review and evidence services preserve project and exact source provenance IDs", async () => {
  calls.length = 0;
  failure = null;
  response = {};
  const reviewRequest = {
    projectId: "project-a",
    title: "Existing-knowledge review",
    searchScope: "Selected source edition",
    sourceId: "source-a",
    sourceVersionId: "version-a",
    sourceLocationId: "location-a",
    sourceClaimId: "claim-a",
    citationNote: "Exact citation",
    createdBy: null,
    reviewerId: null,
  };
  const evidenceRequest = {
    gapId: "gap-a",
    literatureReviewId: "review-a",
    priorAssessmentId: null,
  };

  await researchService.createReviewedLiteratureReview(reviewRequest);
  await researchService.addGapEvidence(evidenceRequest);

  assert.deepEqual(calls, [
    {
      command: "research_create_reviewed_literature_review",
      args: { request: reviewRequest },
    },
    {
      command: "research_add_gap_evidence",
      args: { request: evidenceRequest },
    },
  ]);
});

test("gap evidence service surfaces backend persistence failures", async () => {
  calls.length = 0;
  failure = new Error("evidence belongs to another project");
  const evidenceRequest = {
    gapId: "gap-b",
    literatureReviewId: "review-a",
    priorAssessmentId: null,
  };

  await assert.rejects(
    researchService.addGapEvidence(evidenceRequest),
    /evidence belongs to another project/,
  );
  assert.deepEqual(calls, [{
    command: "research_add_gap_evidence",
    args: { request: evidenceRequest },
  }]);
});
