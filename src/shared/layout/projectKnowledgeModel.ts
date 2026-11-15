import type {
  ResearchProjectSourceDto,
  ResearchWorkspaceDto,
} from "@/types";

export interface ProjectKnowledgeReference {
  kind: "literature_review" | "hypothesis_evidence";
  recordId: string;
  recordTitle: string;
  recordStatus: string;
  sourceId: string | null;
  sourceVersionId: string | null;
  sourceLocationId: string | null;
  sourceClaimId: string | null;
  note: string;
  polarity: string | null;
}

export interface ProjectKnowledgeSource {
  sourceId: string;
  title: string;
  sourceKind: string | null;
  association: ResearchProjectSourceDto | null;
  references: ProjectKnowledgeReference[];
}

export interface ProjectKnowledgeOverview {
  sources: ProjectKnowledgeSource[];
  references: ProjectKnowledgeReference[];
  literatureReviews: ResearchWorkspaceDto["literatureReviews"];
  hypotheses: ResearchWorkspaceDto["hypotheses"];
  questions: ResearchWorkspaceDto["questions"];
  gaps: ResearchWorkspaceDto["gaps"];
  observations: ResearchWorkspaceDto["observations"];
  patterns: ResearchWorkspaceDto["patterns"];
  ideas: ResearchWorkspaceDto["ideas"];
  designVersions: ResearchWorkspaceDto["designVersions"];
  adoptions: ResearchWorkspaceDto["adoptions"];
  availableFrameworkVersions: ResearchWorkspaceDto["frameworkVersions"];
  questionCount: number;
  gapCount: number;
  observationCount: number;
  patternCount: number;
  ideaCount: number;
  designVersionCount: number;
}

export function buildProjectKnowledgeOverview(
  projectId: string,
  workspace: ResearchWorkspaceDto,
  projectSources: ResearchProjectSourceDto[],
): ProjectKnowledgeOverview {
  const literatureReviews = workspace.literatureReviews.filter(
    (review) => review.projectId === projectId,
  );
  const hypotheses = workspace.hypotheses.filter(
    (hypothesis) => hypothesis.projectId === projectId,
  );
  const questions = workspace.questions.filter((question) => question.projectId === projectId);
  const gaps = workspace.gaps.filter((gap) => gap.projectId === projectId);
  const observations = workspace.observations.filter((item) => item.projectId === projectId);
  const patterns = workspace.patterns.filter((item) => item.projectId === projectId);
  const ideas = workspace.ideas.filter((item) => item.projectId === projectId);
  const designVersions = workspace.designVersions.filter((item) => item.projectId === projectId);
  const references: ProjectKnowledgeReference[] = literatureReviews.flatMap((review) =>
    review.sources.map((citation) => ({
      kind: "literature_review",
      recordId: review.id,
      recordTitle: review.title,
      recordStatus: review.lifecycleStatus,
      sourceId: citation.sourceId,
      sourceVersionId: citation.sourceVersionId,
      sourceLocationId: citation.sourceLocationId,
      sourceClaimId: citation.sourceClaimId,
      note: citation.citationNote,
      polarity: null,
    })),
  );

  for (const hypothesis of hypotheses) {
    for (const evidence of hypothesis.evidence) {
      if (evidence.targetKind !== "source_claim") continue;
      references.push({
        kind: "hypothesis_evidence",
        recordId: hypothesis.id,
        recordTitle: `Hypothesis ${hypothesis.id}`,
        recordStatus: hypothesis.lifecycleStatus,
        sourceId: evidence.sourceId,
        sourceVersionId: evidence.sourceVersionId,
        sourceLocationId: evidence.sourceLocationId,
        sourceClaimId: evidence.targetId,
        note: evidence.rationale,
        polarity: evidence.polarity,
      });
    }
  }

  const associations = projectSources.filter((source) => source.projectId === projectId);
  const sourcesById = new Map<string, ProjectKnowledgeSource>();
  for (const association of associations) {
    sourcesById.set(association.sourceId, {
      sourceId: association.sourceId,
      title: association.sourceTitle,
      sourceKind: association.sourceKind,
      association,
      references: [],
    });
  }

  for (const reference of references) {
    if (!reference.sourceId) continue;
    let source = sourcesById.get(reference.sourceId);
    if (!source) {
      const versionReference = workspace.sourceVersions.find(
        (version) =>
          version.sourceId === reference.sourceId
          && version.sourceVersionId === reference.sourceVersionId,
      );
      source = {
        sourceId: reference.sourceId,
        title: versionReference?.sourceTitle ?? reference.sourceId,
        sourceKind: null,
        association: null,
        references: [],
      };
      sourcesById.set(reference.sourceId, source);
    }
    source.references.push(reference);
  }

  return {
    sources: [...sourcesById.values()].sort((left, right) =>
      left.title.localeCompare(right.title) || left.sourceId.localeCompare(right.sourceId),
    ),
    references,
    literatureReviews,
    hypotheses,
    questions,
    gaps,
    observations,
    patterns,
    ideas,
    designVersions,
    adoptions: workspace.adoptions.filter((adoption) => adoption.projectId === projectId),
    availableFrameworkVersions: workspace.frameworkVersions,
    questionCount: questions.length,
    gapCount: gaps.length,
    observationCount: observations.length,
    patternCount: patterns.length,
    ideaCount: ideas.length,
    designVersionCount: designVersions.length,
  };
}
