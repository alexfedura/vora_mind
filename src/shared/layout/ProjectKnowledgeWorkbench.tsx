import React, { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { BookOpen, ExternalLink, RefreshCw, ShieldAlert } from "lucide-react";
import {
  useProjectKnowledgeWorkbench,
  type ProjectKnowledgeLoadStatus,
} from "@/hooks/useProjectKnowledgeWorkbench";
import {
  buildProjectKnowledgeOverview,
  type ProjectKnowledgeReference,
} from "@/shared/layout/projectKnowledgeModel";
import type {
  ResearchProjectSourceDto,
  ResearchWorkspaceDto,
} from "@/types";

type LoadStatus = ProjectKnowledgeLoadStatus;

interface ProjectKnowledgeWorkbenchProps {
  projectId: string;
  projectSources: ResearchProjectSourceDto[];
  projectSourcesStatus: LoadStatus;
  projectSourcesError: string | null;
  workspace: ResearchWorkspaceDto;
}

const panelClass = "rounded-xl border border-[#292c34] bg-[#17191f] p-4";
const mutedClass = "text-xs text-gray-500";
const selectClass =
  "w-full rounded-lg border border-[#343741] bg-[#101114] px-3 py-2 text-sm text-white";

function formatInstant(timestampUs: number): string {
  return new Date(timestampUs / 1000).toLocaleString();
}

function sourceBrowserHref(
  sourceId: string,
  reference?: Partial<Pick<
    ProjectKnowledgeReference,
    "sourceVersionId" | "sourceLocationId" | "sourceClaimId"
  >>,
): string {
  const params = new URLSearchParams({ sourceId });
  if (reference?.sourceVersionId) params.set("sourceVersionId", reference.sourceVersionId);
  if (reference?.sourceLocationId) params.set("sourceLocationId", reference.sourceLocationId);
  if (reference?.sourceClaimId) params.set("sourceClaimId", reference.sourceClaimId);
  return `/sources?${params.toString()}`;
}

function referenceLabel(reference: ProjectKnowledgeReference): string {
  return reference.kind === "literature_review"
    ? `Literature review: ${reference.recordTitle}`
    : `${reference.recordTitle} evidence${reference.polarity ? ` · ${reference.polarity}` : ""}`;
}

export const ProjectKnowledgeWorkbench: React.FC<ProjectKnowledgeWorkbenchProps> = ({
  projectId,
  projectSources,
  projectSourcesStatus,
  projectSourcesError,
  workspace,
}) => {
  const overview = useMemo(
    () => buildProjectKnowledgeOverview(projectId, workspace, projectSources),
    [projectId, projectSources, workspace],
  );
  const [selectedSourceId, setSelectedSourceId] = useState("");
  const [selectedVersionId, setSelectedVersionId] = useState("");
  const [selectedLocationId, setSelectedLocationId] = useState("");
  const [selectedClaimId, setSelectedClaimId] = useState("");
  const [selectedAdoptionId, setSelectedAdoptionId] = useState("");
  const selectedSource = overview.sources.find((source) => source.sourceId === selectedSourceId);
  const selectedAdoption = overview.adoptions.find(
    (adoption) => adoption.id === selectedAdoptionId,
  ) ?? null;
  const {
    sourceData,
    versionData,
    relationData,
    provenanceData: provenanceState,
    frameworkData,
    selectedVersion,
    selectedClaim,
    refreshSource,
    loadProvenance,
    closeProvenance,
  } = useProjectKnowledgeWorkbench({
    sourceId: selectedSourceId,
    sourceVersionId: selectedVersionId,
    sourceClaimId: selectedClaimId,
    frameworkId: selectedAdoption?.frameworkId ?? null,
    frameworkVersionId: selectedAdoption?.frameworkVersionId ?? null,
  });
  const visibleSourceData = sourceData.sourceId === selectedSourceId ? sourceData : null;
  const visibleVersionData = versionData.versionId === selectedVersionId ? versionData : null;
  const exactVersionMissing = Boolean(
    selectedVersionId
    && visibleSourceData?.status === "loaded"
    && !selectedVersion,
  );
  const exactLocationMissing = Boolean(
    selectedLocationId
    && visibleVersionData?.status === "loaded"
    && !visibleVersionData.locations.some((location) => location.id === selectedLocationId),
  );
  const exactClaimMissing = Boolean(
    selectedClaimId
    && visibleSourceData?.status === "loaded"
    && !selectedClaim,
  );

  useEffect(() => {
    if (overview.sources.some((source) => source.sourceId === selectedSourceId)) return;
    setSelectedSourceId(overview.sources[0]?.sourceId ?? "");
    setSelectedVersionId("");
    setSelectedLocationId("");
    setSelectedClaimId("");
  }, [overview.sources, selectedSourceId]);

  useEffect(() => {
    if (overview.adoptions.some((adoption) => adoption.id === selectedAdoptionId)) return;
    setSelectedAdoptionId(overview.adoptions[0]?.id ?? "");
  }, [overview.adoptions, selectedAdoptionId]);

  const openReference = (reference: ProjectKnowledgeReference): void => {
    if (!reference.sourceId) return;
    setSelectedSourceId(reference.sourceId);
    setSelectedVersionId(reference.sourceVersionId ?? "");
    setSelectedLocationId(reference.sourceLocationId ?? "");
    setSelectedClaimId(reference.sourceClaimId ?? "");
    document.getElementById("project-knowledge-source-details")?.scrollIntoView({
      behavior: "smooth",
      block: "start",
    });
  };

  const projectHypothesisEvidence = overview.hypotheses.flatMap((hypothesis) =>
    hypothesis.evidence
      .filter((evidence) => evidence.targetKind === "source_claim")
      .map((evidence) => ({
        hypothesis,
        evidence,
      })),
  );
  const questionGapRecords = [
    ...overview.questions.map((question) => ({
      id: question.id,
      title: question.title,
      status: question.lifecycleStatus,
      detail: question.description,
      kind: "Question",
    })),
    ...overview.gaps.map((gap) => ({
      id: gap.id,
      title: gap.title,
      status: gap.lifecycleStatus,
      detail: gap.statement,
      kind: "Research gap",
    })),
  ];
  const patternIdeaRecords = [
    ...overview.patterns.map((pattern) => ({
      id: pattern.id,
      title: pattern.title,
      detail: pattern.statement,
      kind: "Pattern",
    })),
    ...overview.ideas.map((idea) => ({
      id: idea.id,
      title: idea.title,
      detail: idea.content,
      kind: "Idea",
    })),
  ];
  const selectedRelations =
    relationData.claimId === selectedClaim?.id ? relationData : null;
  const selectedFramework =
    frameworkData.versionId === selectedAdoption?.frameworkVersionId
      ? frameworkData
      : null;
  const selectedFrameworkVersion =
    selectedFramework?.status === "loaded" ? selectedFramework.version : null;

  return (
    <section className="space-y-4 rounded-xl border border-[#3b3428] bg-[#191813] p-4" aria-labelledby="project-knowledge-heading">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <BookOpen size={16} className="text-[#e8ad55]" />
            <h2 id="project-knowledge-heading" className="text-lg font-semibold text-white">
              Project Knowledge
            </h2>
          </div>
          <p className="mt-1 max-w-3xl text-xs text-gray-400">
            Follow explicit Source, Research, and Framework references for this project. Shared Source visibility does not imply project ownership or usage.
          </p>
        </div>
        <Link to="/sources" className="inline-flex items-center gap-1.5 rounded border border-[#49402f] px-3 py-2 text-xs text-amber-200 hover:bg-[#242117]">
          Open shared Source Browser <ExternalLink size={12} />
        </Link>
      </div>

      <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
        <SummaryCard label="Project Source associations" value={projectSources.filter((source) => source.projectId === projectId).length} />
        <SummaryCard label="Exact Research references" value={overview.references.length} />
        <SummaryCard label="Questions / gaps" value={`${overview.questionCount} / ${overview.gapCount}`} />
        <SummaryCard label="Hypotheses / observations" value={`${overview.hypotheses.length} / ${overview.observationCount}`} />
        <SummaryCard label="Patterns / ideas" value={`${overview.patternCount} / ${overview.ideaCount}`} />
        <SummaryCard label="Design versions" value={overview.designVersionCount} />
        <SummaryCard label="FrameworkVersion adoptions" value={overview.adoptions.length} />
      </div>

      <div className="grid gap-4 xl:grid-cols-[minmax(240px,0.8fr)_minmax(0,2fr)]">
        <div className={`${panelClass} space-y-3`}>
          <div className="flex items-center justify-between gap-2">
            <h3 className="text-sm font-semibold text-white">Project-linked Sources</h3>
            <span className={mutedClass}>{overview.sources.length}</span>
          </div>
          {projectSourcesStatus === "loading" && (
            <p className={mutedClass}>Loading project Source associations…</p>
          )}
          {projectSourcesStatus === "error" && (
            <p role="alert" className="text-xs text-rose-300">
              {projectSourcesError ?? "Project Source associations are unavailable."}
            </p>
          )}
          {overview.sources.length === 0 && projectSourcesStatus === "loaded" && (
            <p className={mutedClass}>
              No Sources are associated or cited by this project. A workspace Source is not linked to this project unless a project record explicitly references it.
            </p>
          )}
          <ul className="space-y-2">
            {overview.sources.map((source) => (
              <li key={source.sourceId}>
                <button
                  type="button"
                  onClick={() => {
                    setSelectedSourceId(source.sourceId);
                    setSelectedVersionId("");
                    setSelectedLocationId("");
                    setSelectedClaimId("");
                  }}
                  aria-pressed={selectedSourceId === source.sourceId}
                  className={`w-full rounded-lg border p-3 text-left ${
                    selectedSourceId === source.sourceId
                      ? "border-[#e8ad55]/60 bg-[#252117]"
                      : "border-[#292c34] bg-[#101114] hover:border-[#484b54]"
                  }`}
                >
                  <span className="block text-sm font-medium text-white">{source.title}</span>
                  <span className="mt-1 block break-all font-mono text-[10px] text-gray-500">
                    {source.sourceKind ? `${source.sourceKind} · ` : ""}{source.sourceId}
                  </span>
                  <span className="mt-1 block text-[10px] text-gray-400">
                    {source.association ? "Explicit project association" : "Referenced by project Research"}
                    {" · "}{source.references.length} exact reference(s)
                  </span>
                  {source.association && !source.references.length && (
                    <span className="mt-1 block text-[10px] text-amber-300">
                      Associated context only; no exact SourceVersion citation is recorded.
                    </span>
                  )}
                  {!source.association && (
                    <span className="mt-1 block text-[10px] text-sky-300">
                      Citation does not create a Project Source association.
                    </span>
                  )}
                </button>
              </li>
            ))}
          </ul>
        </div>

        <div id="project-knowledge-source-details" className={`${panelClass} min-w-0 space-y-4`}>
          {!selectedSource ? (
            <p className={mutedClass}>Select an explicitly associated or project-cited Source to inspect its versions and provenance.</p>
          ) : (
            <>
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <h3 className="text-base font-semibold text-white">{selectedSource.title}</h3>
                  <p className="mt-1 break-all font-mono text-[10px] text-gray-500">{selectedSource.sourceId}</p>
                  <p className="mt-1 text-[11px] text-gray-400">
                    {selectedSource.association
                      ? `Project context: ${selectedSource.association.useContext || "not specified"}`
                      : "This Source is linked through Research references, not a Project Source association."}
                  </p>
                </div>
                <button
                  type="button"
                  onClick={refreshSource}
                  disabled={visibleSourceData?.status === "loading"}
                  className="inline-flex items-center gap-1.5 rounded border border-[#343741] px-2.5 py-1.5 text-xs text-gray-300 disabled:opacity-50"
                >
                  <RefreshCw size={12} className={visibleSourceData?.status === "loading" ? "animate-spin" : ""} />
                  Refresh
                </button>
              </div>

              {visibleSourceData?.status === "loading" && <p className={mutedClass}>Loading exact SourceVersions and claims…</p>}
              {visibleSourceData?.status === "error" && (
                <p role="alert" className="text-xs text-rose-300">{visibleSourceData.error}</p>
              )}
              {visibleSourceData?.status === "loaded" && (
                <>
                  <label className="block space-y-1.5 text-xs text-gray-400">
                    <span>Exact SourceVersion</span>
                    <select
                      className={selectClass}
                      value={selectedVersionId}
                      onChange={(event) => {
                        setSelectedVersionId(event.target.value);
                        setSelectedLocationId("");
                        setSelectedClaimId("");
                      }}
                    >
                      <option value="">Select a version; none is inferred from the project association</option>
                      {exactVersionMissing && (
                        <option value={selectedVersionId}>
                          Requested version unavailable: {selectedVersionId}
                        </option>
                      )}
                      {visibleSourceData.versions.map((version) => (
                        <option key={version.id} value={version.id}>
                          {version.editionLabel || version.captureKind} · {version.id}
                        </option>
                      ))}
                    </select>
                  </label>
                  {visibleSourceData.versions.length === 0 && (
                    <p className={mutedClass}>No SourceVersion metadata exists for this Source. Legacy revision history is not substituted for a version.</p>
                  )}
                  {exactVersionMissing && (
                    <p role="alert" className="text-xs text-amber-300">
                      The referenced exact SourceVersion is unavailable. Another version was not selected as a substitute.
                    </p>
                  )}
                </>
              )}

              {selectedVersion && (
                <div className="space-y-4 border-t border-[#292c34] pt-4">
                  <div className="flex flex-wrap items-start justify-between gap-3 rounded-lg bg-[#101114] p-3">
                    <div className="min-w-0">
                      <h4 className="text-sm font-semibold text-white">
                        {selectedVersion.editionLabel || selectedVersion.captureKind}
                      </h4>
                      <p className="mt-1 break-all font-mono text-[10px] text-gray-400">
                        SourceVersion {selectedVersion.id}
                      </p>
                      <p className="mt-1 text-[10px] text-gray-500">
                        Captured {formatInstant(selectedVersion.capturedAtUs)}
                        {selectedVersion.contentSha256 ? ` · SHA-256 ${selectedVersion.contentSha256}` : ""}
                      </p>
                      {selectedVersion.externalAssetRef && (
                        <p className="mt-1 break-all text-[10px] text-gray-500">
                          External asset reference: {selectedVersion.externalAssetRef}
                        </p>
                      )}
                      <p className="mt-1 text-[10px] text-amber-300">
                        Version metadata does not guarantee that source bytes are locally available.
                      </p>
                    </div>
                    <Link
                      to={sourceBrowserHref(selectedSource.sourceId, {
                        sourceVersionId: selectedVersion.id,
                        sourceLocationId: selectedLocationId || null,
                        sourceClaimId: selectedClaimId || null,
                      })}
                      className="shrink-0 rounded border border-[#49402f] px-2.5 py-1.5 text-[11px] text-amber-200 hover:bg-[#242117]"
                    >
                      Open this version in Source Browser
                    </Link>
                  </div>

                  {visibleVersionData?.status === "loading" && <p className={mutedClass}>Loading locations and extractions for this exact version…</p>}
                  {visibleVersionData?.status === "error" && (
                    <p role="alert" className="text-xs text-rose-300">{visibleVersionData.error}</p>
                  )}
                  {exactLocationMissing && (
                    <p role="alert" className="text-xs text-amber-300">
                      The referenced SourceLocation is unavailable for this version; no other location was substituted.
                    </p>
                  )}
                  {visibleVersionData?.status === "loaded" && (
                    <div className="grid gap-4 lg:grid-cols-2">
                      <div className="space-y-2">
                        <h4 className="text-xs font-semibold text-gray-200">
                          SourceLocations · {visibleVersionData.locations.length}
                        </h4>
                        {visibleVersionData.locations.length === 0 ? (
                          <p className={mutedClass}>No locations are recorded for this version.</p>
                        ) : (
                          <ul className="space-y-2">
                            {visibleVersionData.locations.map((location) => (
                              <li key={location.id}>
                                <button
                                  type="button"
                                  onClick={() => setSelectedLocationId(location.id)}
                                  aria-pressed={selectedLocationId === location.id}
                                  className={`w-full rounded border p-2 text-left ${
                                    selectedLocationId === location.id
                                      ? "border-sky-600/70 bg-sky-950/30"
                                      : "border-[#292c34] bg-[#101114]"
                                  }`}
                                >
                                  <span className="block text-xs text-gray-200">{location.label} · {location.locationKind.replace("_", " ")}</span>
                                  <span className="mt-1 block break-all font-mono text-[10px] text-gray-500">
                                    {location.sourceVersionId} / {location.id}
                                  </span>
                                  {location.locatorText && (
                                    <span className="mt-1 block whitespace-pre-wrap text-[11px] text-gray-400">{location.locatorText}</span>
                                  )}
                                  {location.notes && <span className="mt-1 block text-[10px] text-gray-500">{location.notes}</span>}
                                </button>
                                <button
                                  type="button"
                                  onClick={() => void loadProvenance("source_location", location.id)}
                                  className="ml-2 mt-1 text-[10px] text-sky-300"
                                >
                                  Location provenance
                                </button>
                              </li>
                            ))}
                          </ul>
                        )}
                      </div>

                      <div className="space-y-2">
                        <h4 className="text-xs font-semibold text-gray-200">
                          KnowledgeExtractions · {visibleVersionData.extractions.length}
                        </h4>
                        {visibleVersionData.extractions.length === 0 ? (
                          <p className={mutedClass}>No extractions are recorded for this version.</p>
                        ) : (
                          <ul className="space-y-2">
                            {visibleVersionData.extractions.map((extraction) => (
                              <li key={extraction.id} className="rounded border border-[#292c34] bg-[#101114] p-2">
                                <div className="flex flex-wrap items-center justify-between gap-2">
                                  <span className="text-xs text-gray-200">{extraction.kind.replace("_", " ")}</span>
                                  <span className={`rounded px-1.5 py-0.5 text-[10px] ${
                                    extraction.reviewStatus === "accepted"
                                      ? "bg-emerald-950 text-emerald-200"
                                      : extraction.reviewStatus === "rejected"
                                        ? "bg-rose-950 text-rose-200"
                                        : "bg-amber-950 text-amber-200"
                                  }`}>
                                    {extraction.reviewStatus.replace("_", " ")}
                                  </span>
                                </div>
                                <p className="mt-1 whitespace-pre-wrap text-xs text-gray-300">{extraction.content}</p>
                                <p className="mt-1 break-all font-mono text-[10px] text-gray-500">
                                  Version {extraction.sourceVersionId} · location {extraction.sourceSegmentId}
                                </p>
                                <button
                                  type="button"
                                  onClick={() => void loadProvenance("extraction", extraction.id)}
                                  className="mt-1 text-[10px] text-sky-300"
                                >
                                  Extraction provenance
                                </button>
                              </li>
                            ))}
                          </ul>
                        )}
                      </div>
                    </div>
                  )}

                  <div className="space-y-2 border-t border-[#292c34] pt-4">
                    <h4 className="text-xs font-semibold text-gray-200">
                      SourceClaims for this exact version
                    </h4>
                    {visibleSourceData?.claims
                      .filter((claim) => claim.sourceVersionId === selectedVersion.id)
                      .map((claim) => (
                        <article key={claim.id} className="rounded border border-[#292c34] bg-[#101114] p-3">
                          <button
                            type="button"
                            onClick={() => setSelectedClaimId(claim.id)}
                            aria-pressed={selectedClaimId === claim.id}
                            className="text-left text-xs font-medium text-white"
                          >
                            {claim.statement}
                          </button>
                          <p className="mt-1 break-all font-mono text-[10px] text-gray-500">
                            Claim {claim.id} · extraction {claim.extractionId} · SourceVersion {claim.sourceVersionId}
                          </p>
                          <p className="mt-1 text-[10px] text-amber-300">
                            Source-attributed statement; not a verified scientific conclusion.
                          </p>
                          <div className="mt-2 flex flex-wrap gap-3">
                            <button
                              type="button"
                              onClick={() => void loadProvenance("source_claim", claim.id)}
                              className="text-[10px] text-sky-300"
                            >
                              Claim provenance
                            </button>
                            <Link
                              to={sourceBrowserHref(selectedSource.sourceId, {
                                sourceVersionId: claim.sourceVersionId,
                                sourceClaimId: claim.id,
                              })}
                              className="text-[10px] text-amber-200"
                            >
                              Open claim in Source Browser
                            </Link>
                          </div>
                        </article>
                      ))}
                    {(visibleSourceData?.claims.filter((claim) => claim.sourceVersionId === selectedVersion.id).length ?? 0) === 0 && (
                      <p className={mutedClass}>No SourceClaims are recorded for this version.</p>
                    )}
                    {exactClaimMissing && (
                      <p role="alert" className="text-xs text-amber-300">
                        The referenced SourceClaim is unavailable in this source/version context.
                      </p>
                    )}
                    {selectedClaim && (
                      <div className="rounded border border-sky-900/70 bg-sky-950/20 p-3">
                        <h5 className="text-xs font-semibold text-sky-100">Recorded relationships · {selectedClaim.id}</h5>
                        {selectedRelations?.status === "loading" && <p className="mt-2 text-[11px] text-gray-400">Loading recorded relationships…</p>}
                        {selectedRelations?.status === "error" && <p role="alert" className="mt-2 text-[11px] text-rose-300">{selectedRelations.error}</p>}
                        {selectedRelations?.status === "loaded" && selectedRelations.relations.length === 0 && (
                          <p className="mt-2 text-[11px] text-gray-400">No typed relationships are recorded for this claim.</p>
                        )}
                        {selectedRelations?.status === "loaded" && selectedRelations.relations.length > 0 && (
                          <ul className="mt-2 space-y-1">
                            {selectedRelations.relations.map((relation) => (
                              <li key={`${relation.targetKind}:${relation.targetId}:${relation.relationKind}`} className="text-[11px] text-gray-300">
                                {relation.relationKind} → {relation.targetKind} {relation.targetId}
                                {relation.rationale ? ` · ${relation.rationale}` : ""}
                              </li>
                            ))}
                          </ul>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              )}
              {visibleSourceData?.status === "loaded" && !selectedVersion && !exactVersionMissing && (
                <p className={mutedClass}>Choose an exact SourceVersion to inspect its locations, extractions, and claims.</p>
              )}
            </>
          )}
        </div>
      </div>

      <div className="grid gap-4 xl:grid-cols-2">
        <section id="project-knowledge-research" className={`${panelClass} space-y-3`}>
          <div>
            <h3 className="text-sm font-semibold text-white">Research usage and exact references</h3>
            <p className="mt-1 text-[11px] text-gray-500">
              Only this project&apos;s literature citations and project-owned hypothesis evidence are shown. Unassigned records are not included.
            </p>
          </div>
          {overview.references.length === 0 ? (
            <p className={mutedClass}>No project-specific SourceVersion or SourceClaim references are recorded.</p>
          ) : (
            <ul className="space-y-2">
              {overview.references.map((reference, index) => (
                <li key={`${reference.kind}:${reference.recordId}:${reference.sourceClaimId ?? reference.sourceVersionId ?? index}`} className="rounded border border-[#292c34] bg-[#101114] p-3">
                  <div className="flex flex-wrap items-start justify-between gap-2">
                    <div>
                      <p className="text-xs font-medium text-white">{referenceLabel(reference)}</p>
                      <p className="mt-1 text-[10px] text-gray-500">
                        {reference.recordStatus} · record {reference.recordId}
                      </p>
                    </div>
                    {reference.sourceId && (
                      <button
                        type="button"
                        disabled={!reference.sourceVersionId}
                        onClick={() => openReference(reference)}
                        className="rounded border border-[#49402f] px-2 py-1 text-[10px] text-amber-200 disabled:cursor-not-allowed disabled:opacity-40"
                      >
                        Inspect exact Source reference
                      </button>
                    )}
                  </div>
                  <p className="mt-1 break-all font-mono text-[10px] text-gray-400">
                    Source {reference.sourceId ?? "not recorded"}
                    {" · Version "}{reference.sourceVersionId ?? "not recorded"}
                    {" · Location "}{reference.sourceLocationId ?? "not specified"}
                    {" · SourceClaim "}{reference.sourceClaimId ?? "not specified"}
                  </p>
                  {reference.note && <p className="mt-1 text-[11px] text-gray-400">{reference.note}</p>}
                  {!reference.sourceVersionId && (
                    <p className="mt-1 text-[10px] text-amber-300">
                      No exact SourceVersion is recorded for this evidence reference; none is inferred.
                    </p>
                  )}
                </li>
              ))}
            </ul>
          )}
          <details className="rounded border border-[#292c34] bg-[#101114] p-3">
            <summary className="cursor-pointer text-xs text-gray-300">
              Project Research records · {overview.questions.length} questions, {overview.gaps.length} gaps, {overview.hypotheses.length} hypotheses
            </summary>
            <div className="mt-3 space-y-3">
              <div>
                <h4 className="text-[11px] font-semibold text-gray-300">Questions and gaps</h4>
                {questionGapRecords.length === 0 ? (
                  <p className={`mt-1 ${mutedClass}`}>No project Questions or gaps are recorded.</p>
                ) : (
                  <ul className="mt-1 space-y-1">
                    {questionGapRecords.map((record) => (
                      <li key={`${record.kind}:${record.id}`} className="rounded border border-[#292c34] p-2">
                        <p className="text-[11px] text-gray-200">{record.kind}: {record.title} · {record.status}</p>
                        <p className="mt-1 text-[10px] text-gray-400">{record.detail}</p>
                        <p className="mt-1 break-all font-mono text-[10px] text-gray-600">{record.id}</p>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
              <div>
                <h4 className="text-[11px] font-semibold text-gray-300">Hypotheses and definition versions</h4>
                {overview.hypotheses.length === 0 ? (
                  <p className={`mt-1 ${mutedClass}`}>No project-owned Hypotheses are recorded.</p>
                ) : (
                  <ul className="mt-1 space-y-2">
                    {overview.hypotheses.map((hypothesis) => (
                      <li key={hypothesis.id} className="rounded border border-[#292c34] p-2">
                        <p className="text-[11px] text-gray-200">
                          Hypothesis {hypothesis.id} · {hypothesis.lifecycleStatus}
                        </p>
                        {hypothesis.definitions.map((definition) => (
                          <div key={definition.id} className="mt-1 border-l border-[#454955] pl-2">
                            <p className="text-[10px] text-gray-300">
                              Definition v{definition.versionNo} · {definition.lifecycleStatus}
                            </p>
                            <p className="mt-1 text-[10px] text-gray-400">{definition.statementH1}</p>
                            <p className="mt-1 break-all font-mono text-[10px] text-gray-600">{definition.id}</p>
                          </div>
                        ))}
                      </li>
                    ))}
                  </ul>
                )}
              </div>
              <div className="grid gap-3 md:grid-cols-2">
                <div>
                  <h4 className="text-[11px] font-semibold text-gray-300">Patterns and ideas</h4>
                  {patternIdeaRecords.length === 0 ? (
                    <p className={`mt-1 ${mutedClass}`}>No project Patterns or Ideas are recorded.</p>
                  ) : (
                    <ul className="mt-1 space-y-1">
                      {patternIdeaRecords.map((record) => (
                        <li key={`${record.kind}:${record.id}`} className="rounded border border-[#292c34] p-2">
                          <p className="text-[10px] text-gray-300">{record.kind}: {record.title}</p>
                          <p className="mt-1 text-[10px] text-gray-500">{record.detail}</p>
                        </li>
                      ))}
                    </ul>
                  )}
                  <p className="mt-2 text-[10px] text-gray-500">
                    {overview.observations.length} project Observation record(s) · no measurement or conclusion is inferred here.
                  </p>
                </div>
                <div>
                  <h4 className="text-[11px] font-semibold text-gray-300">Research design versions</h4>
                  {overview.designVersions.length === 0 ? (
                    <p className={`mt-1 ${mutedClass}`}>No project Research design versions are recorded.</p>
                  ) : (
                    <ul className="mt-1 space-y-1">
                      {overview.designVersions.map((design) => (
                        <li key={design.id} className="rounded border border-[#292c34] p-2">
                          <p className="text-[10px] text-gray-300">{design.title} · v{design.versionNo} · {design.lifecycleStatus}</p>
                          <p className="mt-1 break-all font-mono text-[10px] text-gray-600">{design.id}</p>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              </div>
            </div>
          </details>
          <div className="flex flex-wrap gap-3 border-t border-[#292c34] pt-3">
            <a href="#research-literature" className="text-[11px] text-sky-300">Open Research literature reviews</a>
            <a href="#research-question-lifecycle" className="text-[11px] text-sky-300">Open project Questions</a>
          </div>
        </section>

        <section id="project-knowledge-frameworks" className={`${panelClass} space-y-3`}>
          <div>
            <h3 className="text-sm font-semibold text-white">FrameworkVersions</h3>
            <p className="mt-1 text-[11px] text-gray-500">
              Shared published versions are separate from this project&apos;s explicit adoptions. Publication does not imply transfer eligibility.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <a href="#project-framework-adoption" className="text-[11px] text-sky-300">Adopt an available version</a>
            <span className="text-[11px] text-gray-600">
              {overview.availableFrameworkVersions.length} shared published version(s) available
            </span>
          </div>
          {overview.adoptions.length === 0 ? (
            <p className={mutedClass}>This project has no FrameworkVersion adoption. No ownership or transfer is inferred.</p>
          ) : (
            <div className="space-y-2">
              {overview.adoptions.map((adoption) => (
                <button
                  key={adoption.id}
                  type="button"
                  onClick={() => setSelectedAdoptionId(adoption.id)}
                  aria-pressed={selectedAdoptionId === adoption.id}
                  className={`w-full rounded border p-3 text-left ${
                    selectedAdoptionId === adoption.id
                      ? "border-[#e8ad55]/60 bg-[#252117]"
                      : "border-[#292c34] bg-[#101114]"
                  }`}
                >
                  <span className="block text-xs font-medium text-white">{adoption.title} · v{adoption.versionNo}</span>
                  <span className="mt-1 block break-all font-mono text-[10px] text-gray-500">
                    Framework {adoption.frameworkId} · exact version {adoption.frameworkVersionId}
                  </span>
                  <span className="mt-1 block text-[11px] text-gray-400">{adoption.useContext}</span>
                </button>
              ))}
            </div>
          )}
          {selectedFramework?.status === "loading" && <p className={mutedClass}>Loading the adopted exact FrameworkVersion…</p>}
          {selectedFramework?.status === "error" && (
            <p role="alert" className="text-xs text-rose-300">{selectedFramework.error}</p>
          )}
          {selectedFrameworkVersion && (
            <div className="rounded border border-[#292c34] bg-[#101114] p-3">
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div>
                  <h4 className="text-xs font-semibold text-white">
                    {selectedFrameworkVersion.title} · v{selectedFrameworkVersion.versionNo}
                  </h4>
                  <p className="mt-1 break-all font-mono text-[10px] text-gray-500">
                    {selectedFrameworkVersion.id} · {selectedFrameworkVersion.status}
                  </p>
                </div>
                <button
                  type="button"
                  onClick={() => void loadProvenance("framework_version", selectedFrameworkVersion.id)}
                  className="text-[10px] text-sky-300"
                >
                  Publication provenance
                </button>
              </div>
              {selectedFrameworkVersion.items.length === 0 ? (
                <p className={`mt-2 ${mutedClass}`}>No members are recorded in this version.</p>
              ) : (
                <ol className="mt-2 space-y-2">
                  {selectedFrameworkVersion.items.map((item) => {
                    const citedByResearch = overview.references.some(
                      (reference) => reference.sourceClaimId === item.itemId,
                    ) || projectHypothesisEvidence.some(
                      ({ evidence }) => evidence.targetId === item.itemId,
                    );
                    const sourceReference = overview.references.find(
                      (reference) =>
                        item.itemKind === "source_claim"
                        && reference.sourceClaimId === item.itemId
                        && reference.sourceId
                        && reference.sourceVersionId,
                    );
                    return (
                      <li key={item.id} className="rounded border border-[#292c34] p-2">
                        <p className="text-[10px] text-sky-300">
                          {item.itemOrder + 1}. {item.itemKind} · {item.membershipRole}
                          {citedByResearch ? " · referenced by this project's Research" : ""}
                        </p>
                        <p className="mt-1 text-xs text-gray-200">{item.itemTitle}</p>
                        <p className="mt-1 break-all font-mono text-[10px] text-gray-500">{item.itemId}</p>
                        {item.rationale && <p className="mt-1 text-[10px] text-gray-400">{item.rationale}</p>}
                        {sourceReference && (
                          <button
                            type="button"
                            onClick={() => openReference(sourceReference)}
                            className="mt-2 text-[10px] text-amber-200"
                          >
                            Inspect matching exact Research source reference
                          </button>
                        )}
                      </li>
                    );
                  })}
                </ol>
              )}
            </div>
          )}
          {overview.availableFrameworkVersions.length > 0 && (
            <details className="rounded border border-[#292c34] bg-[#101114] p-3">
              <summary className="cursor-pointer text-xs text-gray-300">
                Shared published versions available to adopt ({overview.availableFrameworkVersions.length})
              </summary>
              <ul className="mt-2 space-y-1">
                {overview.availableFrameworkVersions.map((version) => (
                  <li key={version.frameworkVersionId} className="break-all text-[10px] text-gray-400">
                    {version.title} · v{version.versionNo} · Framework {version.frameworkId} · {version.frameworkVersionId}
                  </li>
                ))}
              </ul>
            </details>
          )}
        </section>
      </div>

      {provenanceState && (
        <section className="rounded-lg border border-sky-900/70 bg-sky-950/20 p-3" aria-live="polite">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h3 className="text-xs font-semibold text-sky-100">
              Provenance · {provenanceState.targetKind} {provenanceState.targetId}
            </h3>
            <button type="button" onClick={closeProvenance} className="text-[10px] text-gray-400">
              Close
            </button>
          </div>
          {provenanceState.status === "loading" && <p className="mt-2 text-[11px] text-gray-400">Loading recorded provenance…</p>}
          {provenanceState.status === "error" && <p role="alert" className="mt-2 text-[11px] text-rose-300">{provenanceState.error}</p>}
          {provenanceState.status === "loaded" && provenanceState.events.length === 0 && (
            <p className="mt-2 text-[11px] text-gray-400">No provenance events are recorded for this item.</p>
          )}
          {provenanceState.status === "loaded" && provenanceState.events.length > 0 && (
            <ol className="mt-2 space-y-2">
              {provenanceState.events.map((event) => (
                <li key={event.id} className="border-l border-sky-700 pl-2 text-[11px]">
                  <span className="text-gray-200">{event.eventKind}</span>
                  <span className="ml-2 text-gray-500">{formatInstant(event.occurredAtUs)}</span>
                  {event.reason && <p className="mt-1 text-gray-400">{event.reason}</p>}
                </li>
              ))}
            </ol>
          )}
        </section>
      )}

      <p className="flex items-start gap-2 text-[10px] text-gray-600">
        <ShieldAlert size={12} className="mt-0.5 shrink-0" />
        These project filters organize a trusted local single-user UI; they are not authentication or authorization boundaries.
      </p>
    </section>
  );
};

function SummaryCard({ label, value }: { label: string; value: string | number }): React.JSX.Element {
  return (
    <div className="rounded-lg border border-[#292c34] bg-[#101114] px-3 py-2">
      <p className="text-[10px] text-gray-500">{label}</p>
      <p className="mt-1 text-sm font-semibold text-white">{value}</p>
    </div>
  );
}
