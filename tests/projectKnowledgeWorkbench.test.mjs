import assert from "node:assert/strict";
import test from "node:test";
import { buildProjectKnowledgeOverview } from "../src/shared/layout/projectKnowledgeModel.ts";

test("the project overview keeps associations separate from exact Research references", () => {
  const projectId = "project-a";
  const workspace = {
    literatureReviews: [
      {
        id: "review-a",
        projectId,
        title: "Review A",
        lifecycleStatus: "reviewed",
        sources: [{
          id: "citation-a",
          sourceId: "source-cited",
          sourceVersionId: "version-exact",
          sourceLocationId: "location-exact",
          sourceClaimId: "claim-exact",
          citationNote: "Specific edition",
        }],
      },
      {
        id: "review-b",
        projectId: "project-b",
        title: "Other review",
        lifecycleStatus: "reviewed",
        sources: [],
      },
    ],
    hypotheses: [
      {
        id: "hypothesis-a",
        projectId,
        lifecycleStatus: "active",
        evidence: [
          {
            targetKind: "source_claim",
            targetId: "claim-hypothesis",
            polarity: "supports",
            rationale: "Recorded claim link",
            sourceId: "source-cited",
            sourceVersionId: "version-hypothesis",
            sourceLocationId: null,
          },
          {
            targetKind: "source_claim",
            targetId: "claim-unresolved",
            polarity: "contradicts",
            rationale: "No SourceVersion identity recorded",
            sourceId: null,
            sourceVersionId: null,
            sourceLocationId: null,
          },
          {
            targetKind: "observation",
            targetId: "observation-1",
            polarity: "supports",
            rationale: "Not a SourceClaim",
            sourceId: null,
            sourceVersionId: null,
            sourceLocationId: null,
          },
        ],
      },
      {
        id: "hypothesis-b",
        projectId: null,
        lifecycleStatus: "active",
        evidence: [],
      },
    ],
    sourceVersions: [
      {
        sourceId: "source-cited",
        sourceVersionId: "version-exact",
        sourceTitle: "Cited source",
      },
      {
        sourceId: "source-cited",
        sourceVersionId: "version-hypothesis",
        sourceTitle: "Cited source",
      },
    ],
    adoptions: [
      { id: "adoption-a", projectId },
      { id: "adoption-b", projectId: "project-b" },
    ],
    frameworkVersions: [],
    questions: [{ projectId }, { projectId: "project-b" }, { projectId: null }],
    gaps: [{ projectId }],
    observations: [{ projectId }],
    patterns: [{ projectId }],
    ideas: [{ projectId }],
    designVersions: [{ projectId }],
  };
  const projectSources = [
    {
      projectId,
      sourceId: "source-associated",
      sourceTitle: "Associated only",
      sourceKind: "book",
      useContext: "Background",
    },
    {
      projectId: "project-b",
      sourceId: "source-other-project",
      sourceTitle: "Other project",
      sourceKind: "article",
    },
  ];

  const overview = buildProjectKnowledgeOverview(projectId, workspace, projectSources);

  assert.deepEqual(
    overview.sources.map(({ sourceId, association, references }) => ({
      sourceId,
      associated: association !== null,
      references: references.map(({ sourceVersionId, sourceLocationId, sourceClaimId }) => ({
        sourceVersionId,
        sourceLocationId,
        sourceClaimId,
      })),
    })),
    [
      {
        sourceId: "source-associated",
        associated: true,
        references: [],
      },
      {
        sourceId: "source-cited",
        associated: false,
        references: [
          {
            sourceVersionId: "version-exact",
            sourceLocationId: "location-exact",
            sourceClaimId: "claim-exact",
          },
          {
            sourceVersionId: "version-hypothesis",
            sourceLocationId: null,
            sourceClaimId: "claim-hypothesis",
          },
        ],
      },
    ],
  );
  assert.equal(overview.references.length, 3);
  assert.equal(overview.references[2].sourceId, null);
  assert.equal(overview.references[2].sourceVersionId, null);
  assert.deepEqual(overview.hypotheses.map(({ id }) => id), ["hypothesis-a"]);
  assert.deepEqual(overview.literatureReviews.map(({ id }) => id), ["review-a"]);
  assert.deepEqual(overview.adoptions.map(({ id }) => id), ["adoption-a"]);
  assert.equal(overview.questionCount, 1);
  assert.equal(overview.gapCount, 1);
});
