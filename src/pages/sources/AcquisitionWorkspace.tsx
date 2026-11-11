import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  srcAddFrameworkItem,
  srcCorrectKnowledgeExtraction,
  srcCreateClaimRelation,
  srcCreateFramework,
  srcCreateFrameworkVersion,
  srcCreateKnowledgeExtraction,
  srcCreateKnowledgeItem,
  srcCreateSourceClaim,
  srcCreateSourceLocation,
  srcCreateSourceVersion,
  srcListClaimRelations,
  srcListFrameworkVersions,
  srcListFrameworks,
  srcListKnowledgeExtractions,
  srcListKnowledgeItems,
  srcListProvenance,
  srcListSourceClaims,
  srcListSourceLocations,
  srcListSourceVersions,
  srcPublishFrameworkVersion,
  srcRemoveFrameworkItem,
  srcReviewFrameworkVersion,
  srcReviewKnowledgeExtraction,
  srcUpdateFrameworkVersion,
  type SourceRecord,
  type SegmentNode,
} from "@/source/services/sourceService";
import type {
  ClaimRelationDto,
  ClaimRelationKind,
  ExtractionKind,
  ExtractionReviewStatus,
  FrameworkChangeClass,
  FrameworkDto,
  FrameworkMembershipRole,
  FrameworkReviewStatus,
  FrameworkVersionDto,
  KnowledgeExtractionDto,
  KnowledgeItemDto,
  KnowledgeItemKind,
  ProvenanceEventDto,
  SourceClaimDto,
  SourceLocationDto,
  SourceLocationKind,
  SourceVersionDto,
} from "@/types";

type WorkspaceTab = "versions" | "knowledge" | "frameworks";

interface AcquisitionWorkspaceProps {
  source: SourceRecord;
  segments: SegmentNode[];
}

interface LegacySegmentOption {
  id: string;
  label: string;
}

const extractionKinds: ExtractionKind[] = [
  "quote",
  "paraphrase",
  "summary",
  "learner_interpretation",
  "source_claim",
];
const locationKinds: SourceLocationKind[] = [
  "page",
  "paragraph",
  "line",
  "timestamp",
  "time_range",
  "text_anchor",
  "unknown",
];
const itemKinds: Exclude<KnowledgeItemKind, "source_claim">[] = [
  "concept",
  "principle",
  "procedure",
];
const membershipRoles: FrameworkMembershipRole[] = [
  "core",
  "supporting",
  "counterpoint",
  "method",
  "example",
  "context",
];
const changeClasses: FrameworkChangeClass[] = [
  "refinement",
  "extension",
  "revision",
  "replacement",
];

function flattenSegments(
  nodes: SegmentNode[],
  depth = 0,
): LegacySegmentOption[] {
  return nodes.flatMap((node) => [
    { id: node.id, label: `${"  ".repeat(depth)}${node.label}` },
    ...flattenSegments(node.children, depth + 1),
  ]);
}

function optionalInteger(value: string): number | null {
  if (!value.trim()) return null;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) ? parsed : null;
}

function formatInstant(value: number): string {
  return new Date(value / 1000).toLocaleString();
}

function locationDescription(location: SourceLocationDto): string {
  switch (location.locationKind) {
    case "page":
      return `Page ${location.pageStart ?? "?"}${location.pageEnd && location.pageEnd !== location.pageStart ? `–${location.pageEnd}` : ""}`;
    case "paragraph":
      return `Paragraph ${location.paragraphStart ?? "?"}`;
    case "line":
      return `Line ${location.lineStart ?? "?"}${location.lineEnd && location.lineEnd !== location.lineStart ? `–${location.lineEnd}` : ""}`;
    case "timestamp":
      return `At ${location.timeStartMs ?? "?"} ms`;
    case "time_range":
      return `${location.timeStartMs ?? "?"}–${location.timeEndMs ?? "?"} ms`;
    case "text_anchor":
      return location.stableSelector ?? "Text anchor";
    case "unknown":
      return "Location unknown";
  }
}

function citedUrl(source: SourceRecord, location: SourceLocationDto): string | null {
  if (!source.path || !/^https?:\/\//i.test(source.path)) return null;
  try {
    const url = new URL(source.path);
    if (location.pageStart != null) {
      url.hash = `page=${location.pageStart}`;
    } else if (location.timeStartMs != null) {
      const start = location.timeStartMs / 1000;
      const end = location.timeEndMs == null ? "" : `,${location.timeEndMs / 1000}`;
      url.hash = `t=${start}${end}`;
    } else if (location.locatorText) {
      url.hash = `:~:text=${encodeURIComponent(location.locatorText)}`;
    }
    return url.toString();
  } catch {
    return null;
  }
}

function claimRelationTargetKind(
  relation: ClaimRelationDto,
): KnowledgeItemKind {
  return relation.targetKind;
}

export const AcquisitionWorkspace: React.FC<AcquisitionWorkspaceProps> = ({
  source,
  segments,
}) => {
  const [tab, setTab] = useState<WorkspaceTab>("versions");
  const [versions, setVersions] = useState<SourceVersionDto[]>([]);
  const [versionId, setVersionId] = useState("");
  const [locations, setLocations] = useState<SourceLocationDto[]>([]);
  const [extractions, setExtractions] = useState<KnowledgeExtractionDto[]>([]);
  const [claims, setClaims] = useState<SourceClaimDto[]>([]);
  const [items, setItems] = useState<KnowledgeItemDto[]>([]);
  const [frameworks, setFrameworks] = useState<FrameworkDto[]>([]);
  const [frameworkId, setFrameworkId] = useState("");
  const [frameworkVersions, setFrameworkVersions] = useState<FrameworkVersionDto[]>([]);
  const [frameworkVersionId, setFrameworkVersionId] = useState("");
  const [compareVersionId, setCompareVersionId] = useState("");
  const [selectedLocationId, setSelectedLocationId] = useState("");
  const [selectedExtractionId, setSelectedExtractionId] = useState("");
  const [selectedClaimId, setSelectedClaimId] = useState("");
  const [selectedItemId, setSelectedItemId] = useState("");
  const [relations, setRelations] = useState<ClaimRelationDto[]>([]);
  const [provenance, setProvenance] = useState<ProvenanceEventDto[]>([]);
  const [provenanceTarget, setProvenanceTarget] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const [versionLabel, setVersionLabel] = useState("");
  const [checksum, setChecksum] = useState("");
  const [assetRef, setAssetRef] = useState("");
  const [locationLabel, setLocationLabel] = useState("");
  const [locationKind, setLocationKind] = useState<SourceLocationKind>("page");
  const [locationStart, setLocationStart] = useState("");
  const [locationEnd, setLocationEnd] = useState("");
  const [locationSelector, setLocationSelector] = useState("");
  const [locationText, setLocationText] = useState("");
  const [legacySegmentId, setLegacySegmentId] = useState("");
  const [extractionKind, setExtractionKind] = useState<ExtractionKind>("quote");
  const [extractionText, setExtractionText] = useState("");
  const [toolIdentity, setToolIdentity] = useState("");
  const [toolVersion, setToolVersion] = useState("");
  const [promptIdentity, setPromptIdentity] = useState("");
  const [correctionText, setCorrectionText] = useState("");
  const [correctionReason, setCorrectionReason] = useState("");
  const [claimDraft, setClaimDraft] = useState("");
  const [knowledgeKind, setKnowledgeKind] =
    useState<Exclude<KnowledgeItemKind, "source_claim">>("concept");
  const [knowledgeTitle, setKnowledgeTitle] = useState("");
  const [knowledgeContent, setKnowledgeContent] = useState("");
  const [relationTargetId, setRelationTargetId] = useState("");
  const [relationKind, setRelationKind] = useState<ClaimRelationKind>("contradicts");
  const [frameworkTitle, setFrameworkTitle] = useState("");
  const [changeClass, setChangeClass] = useState<FrameworkChangeClass>("refinement");
  const [changeRationale, setChangeRationale] = useState("");
  const [affectedClaimIds, setAffectedClaimIds] = useState<string[]>([]);
  const [assumptionClaimIds, setAssumptionClaimIds] = useState<string[]>([]);
  const [frameworkDraftTitle, setFrameworkDraftTitle] = useState("");
  const [frameworkDraftDescription, setFrameworkDraftDescription] = useState("");
  const [frameworkItemKind, setFrameworkItemKind] =
    useState<KnowledgeItemKind>("source_claim");
  const [frameworkItemId, setFrameworkItemId] = useState("");
  const [frameworkRole, setFrameworkRole] = useState<FrameworkMembershipRole>("core");
  const [reviewStatus, setReviewStatus] = useState<FrameworkReviewStatus>("approved");
  const [reviewRationale, setReviewRationale] = useState("");

  const flatSegments = useMemo(() => flattenSegments(segments), [segments]);
  const selectedVersion = versions.find((version) => version.id === versionId) ?? null;
  const selectedFramework = frameworks.find((framework) => framework.id === frameworkId) ?? null;
  const selectedFrameworkVersion =
    frameworkVersions.find((version) => version.id === frameworkVersionId) ?? null;
  const comparedVersion =
    frameworkVersions.find((version) => version.id === compareVersionId) ?? null;
  const selectedClaim = claims.find((claim) => claim.id === selectedClaimId) ?? null;
  const selectedExtraction =
    extractions.find((extraction) => extraction.id === selectedExtractionId) ?? null;

  const loadVersionDetails = useCallback(async (id: string) => {
    if (!id) {
      setLocations([]);
      setExtractions([]);
      return;
    }
    const [nextLocations, nextExtractions] = await Promise.all([
      srcListSourceLocations(id),
      srcListKnowledgeExtractions(id),
    ]);
    setLocations(nextLocations);
    setExtractions(nextExtractions);
  }, []);

  const loadSourceData = useCallback(async () => {
    const [nextVersions, nextClaims, nextItems, nextFrameworks] = await Promise.all([
      srcListSourceVersions(source.id),
      srcListSourceClaims(source.id),
      srcListKnowledgeItems(),
      srcListFrameworks(),
    ]);
    setVersions(nextVersions);
    setClaims(nextClaims);
    setItems(nextItems);
    setFrameworks(nextFrameworks);
    setVersionId((current) =>
      nextVersions.some((version) => version.id === current)
        ? current
        : nextVersions[0]?.id ?? "",
    );
    setFrameworkId((current) =>
      nextFrameworks.some((framework) => framework.id === current)
        ? current
        : nextFrameworks[0]?.id ?? "",
    );
  }, [source.id]);

  const loadFrameworkVersions = useCallback(async (id: string) => {
    if (!id) {
      setFrameworkVersions([]);
      setFrameworkVersionId("");
      return;
    }
    const result = await srcListFrameworkVersions(id);
    setFrameworkVersions(result);
    setFrameworkVersionId((current) =>
      result.some((version) => version.id === current)
        ? current
        : result[0]?.id ?? "",
    );
    setCompareVersionId((current) =>
      result.some((version) => version.id === current)
        ? current
        : result[1]?.id ?? result[0]?.id ?? "",
    );
  }, []);

  useEffect(() => {
    setError(null);
    loadSourceData().catch((loadError: unknown) => setError(String(loadError)));
  }, [loadSourceData]);

  useEffect(() => {
    if (versionId) {
      loadVersionDetails(versionId).catch((loadError: unknown) => setError(String(loadError)));
    }
  }, [versionId, loadVersionDetails]);

  useEffect(() => {
    loadFrameworkVersions(frameworkId).catch((loadError: unknown) =>
      setError(String(loadError)),
    );
  }, [frameworkId, loadFrameworkVersions]);

  useEffect(() => {
    if (selectedFrameworkVersion) {
      setFrameworkDraftTitle(selectedFrameworkVersion.title);
      setFrameworkDraftDescription(selectedFrameworkVersion.description);
    }
  }, [selectedFrameworkVersion?.id]);

  useEffect(() => {
    if (!selectedClaimId) {
      setRelations([]);
      return;
    }
    srcListClaimRelations(selectedClaimId)
      .then(setRelations)
      .catch((loadError: unknown) => setError(String(loadError)));
  }, [selectedClaimId]);

  const runAction = async (action: () => Promise<void>, successMessage: string) => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      await action();
      await loadSourceData();
      await loadVersionDetails(versionId);
      if (frameworkId) await loadFrameworkVersions(frameworkId);
      setNotice(successMessage);
    } catch (actionError) {
      setError(String(actionError));
    } finally {
      setBusy(false);
    }
  };

  const showProvenance = async (kind: string, id: string) => {
    setError(null);
    setProvenanceTarget(`${kind}: ${id}`);
    try {
      setProvenance(await srcListProvenance(kind, id));
    } catch (loadError) {
      setError(String(loadError));
      setProvenance([]);
    }
  };

  const addVersion = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    void runAction(async () => {
      await srcCreateSourceVersion({
        sourceId: source.id,
        captureKind: "edition",
        editionLabel: versionLabel.trim() || null,
        canonicalUri: source.path,
        capturedAtUs: null,
        contentSha256: checksum.trim() || null,
        externalAssetRef: assetRef.trim() || null,
        metadataJson: "{}",
      });
      setVersionLabel("");
      setChecksum("");
      setAssetRef("");
    }, "Source version saved.");
  };

  const addLocation = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedVersion) return;
    const primaryValue = optionalInteger(locationStart);
    const endValue = optionalInteger(locationEnd);
    if (locationStart.trim() && primaryValue == null) {
      setError("Location start must be a whole number.");
      return;
    }
    if (locationEnd.trim() && endValue == null) {
      setError("Location end must be a whole number.");
      return;
    }
    if (
      ["page", "paragraph", "line", "timestamp"].includes(locationKind) &&
      primaryValue == null
    ) {
      setError("This citation location requires a start value.");
      return;
    }
    if (locationKind === "time_range" && (primaryValue == null || endValue == null)) {
      setError("Time ranges require both a start and end in milliseconds.");
      return;
    }
    if (locationKind === "text_anchor" && !locationSelector.trim()) {
      setError("Text anchors require a stable selector.");
      return;
    }
    void runAction(async () => {
      await srcCreateSourceLocation({
        sourceId: source.id,
        sourceVersionId: selectedVersion.id,
        legacySegmentId: legacySegmentId || null,
        label: locationLabel.trim(),
        locationKind,
        pageStart: locationKind === "page" ? primaryValue : null,
        pageEnd: locationKind === "page" ? endValue : null,
        paragraphStart: locationKind === "paragraph" ? primaryValue : null,
        paragraphEnd: locationKind === "paragraph" ? endValue : null,
        lineStart: locationKind === "line" ? primaryValue : null,
        lineEnd: locationKind === "line" ? endValue : null,
        timeStartMs:
          locationKind === "timestamp" || locationKind === "time_range"
            ? primaryValue
            : null,
        timeEndMs: locationKind === "time_range" ? endValue : null,
        stableSelector: locationKind === "text_anchor" ? locationSelector.trim() : null,
        locatorText: locationText.trim() || null,
        notes: null,
        parentSegmentId: null,
        orderIndex: locations.length,
      });
      setLocationLabel("");
      setLocationStart("");
      setLocationEnd("");
      setLocationSelector("");
      setLocationText("");
      setLegacySegmentId("");
    }, "Version-pinned citation location saved.");
  };

  const addExtraction = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedVersion || !selectedLocationId) return;
    void runAction(async () => {
      const created = await srcCreateKnowledgeExtraction({
        sourceId: source.id,
        sourceVersionId: selectedVersion.id,
        sourceSegmentId: selectedLocationId,
        kind: extractionKind,
        content: extractionText.trim(),
        language: source.language,
        parentExtractionId: null,
        toolIdentity: toolIdentity.trim() || null,
        toolVersion: toolVersion.trim() || null,
        promptConfigIdentity: promptIdentity.trim() || null,
        confidenceDecimal: null,
        uncertainty: null,
        createdBy: toolIdentity.trim() ? `machine:${toolIdentity.trim()}` : "user",
      });
      setSelectedExtractionId(created.id);
      setExtractionText("");
    }, "Extraction saved as unreviewed.");
  };

  const reviewExtraction = (
    extractionId: string,
    status: Exclude<ExtractionReviewStatus, "unreviewed">,
  ) => {
    void runAction(async () => {
      await srcReviewKnowledgeExtraction({
        extractionId,
        reviewStatus: status,
        reviewerId: "user",
        rationale: status === "accepted" ? "Extraction quality reviewed." : "",
      });
    }, `Extraction marked ${status.replace("_", " ")}. This does not verify its scientific truth.`);
  };

  const correctExtraction = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedExtraction) return;
    void runAction(async () => {
      const replacement = await srcCorrectKnowledgeExtraction({
        extractionId: selectedExtraction.id,
        content: correctionText.trim(),
        reason: correctionReason.trim(),
        actorId: "user",
      });
      setSelectedExtractionId(replacement.id);
      setCorrectionText("");
      setCorrectionReason("");
    }, "Correction saved as a new extraction; the prior extraction remains in history.");
  };

  const createClaim = (extraction: KnowledgeExtractionDto) => {
    void runAction(async () => {
      await srcCreateSourceClaim({
        extractionId: extraction.id,
        statement: claimDraft.trim() || extraction.content,
        assertedBy: source.author,
        scope: null,
      });
      setClaimDraft("");
    }, "Source-attributed claim saved. Extraction review is not a truth assessment.");
  };

  const createKnowledgeItem = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedClaim) return;
    void runAction(async () => {
      await srcCreateKnowledgeItem({
        kind: knowledgeKind,
        title: knowledgeTitle.trim(),
        content: knowledgeContent.trim(),
        sourceClaimId: selectedClaim.id,
      });
      setKnowledgeTitle("");
      setKnowledgeContent("");
    }, `${knowledgeKind} saved with source-claim provenance.`);
  };

  const addClaimRelation = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedClaim || !relationTargetId) return;
    void runAction(async () => {
      await srcCreateClaimRelation({
        sourceClaimId: selectedClaim.id,
        targetKind: "source_claim",
        targetId: relationTargetId,
        relationKind,
        rationale: "Source claims may conflict; neither assertion is removed.",
        createdBy: "user",
      });
    }, "Typed claim relationship saved.");
  };

  const createFramework = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    void runAction(async () => {
      const created = await srcCreateFramework({
        title: frameworkTitle.trim(),
        description: "",
        createdBy: "user",
      });
      setFrameworkId(created.id);
      setFrameworkTitle("");
      setTab("frameworks");
    }, "Framework draft created.");
  };

  const createNextFrameworkVersion = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedFramework || affectedClaimIds.length + assumptionClaimIds.length === 0) {
      setError("Select at least one affected claim or assumption.");
      return;
    }
    void runAction(async () => {
      const created = await srcCreateFrameworkVersion({
        frameworkId: selectedFramework.id,
        changeClass,
        changeRationale: changeRationale.trim(),
        affectedClaimIds,
        assumptionClaimIds,
        createdBy: "user",
      });
      setFrameworkVersionId(created.id);
      setChangeRationale("");
      setAffectedClaimIds([]);
      setAssumptionClaimIds([]);
    }, "New framework draft created from the latest published version.");
  };

  const updateFrameworkDraft = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedFrameworkVersion) return;
    void runAction(async () => {
      await srcUpdateFrameworkVersion({
        frameworkVersionId: selectedFrameworkVersion.id,
        title: frameworkDraftTitle.trim(),
        description: frameworkDraftDescription,
      });
    }, "Framework draft updated.");
  };

  const addFrameworkItem = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selectedFrameworkVersion || !frameworkItemId) return;
    void runAction(async () => {
      await srcAddFrameworkItem({
        frameworkVersionId: selectedFrameworkVersion.id,
        itemKind: frameworkItemKind,
        itemId: frameworkItemId,
        membershipRole: frameworkRole,
        itemOrder: selectedFrameworkVersion.items.length,
        rationale: null,
      });
    }, "Framework membership added.");
  };

  const latestVersion = frameworkVersions[0] ?? null;
  const canCreateSuccessor = latestVersion?.status === "published";
  const membershipKey = (item: FrameworkVersionDto["items"][number]) =>
    `${item.itemKind}:${item.itemId}`;
  const oldMemberships = new Map(
    (comparedVersion?.items ?? []).map((item) => [membershipKey(item), item]),
  );
  const currentMemberships = new Map(
    (selectedFrameworkVersion?.items ?? []).map((item) => [membershipKey(item), item]),
  );
  const addedItems =
    selectedFrameworkVersion?.items.filter((item) => !oldMemberships.has(membershipKey(item))) ?? [];
  const removedItems =
    comparedVersion?.items.filter((item) => !currentMemberships.has(membershipKey(item))) ?? [];
  const changedItems =
    selectedFrameworkVersion?.items.filter((item) => {
      const previous = oldMemberships.get(membershipKey(item));
      return previous != null &&
        (previous.membershipRole !== item.membershipRole || previous.itemOrder !== item.itemOrder);
    }) ?? [];

  const renderProvenance = () =>
    provenanceTarget ? (
      <div className="mt-4 rounded-lg border border-sky-800/60 bg-sky-950/20 p-3">
        <div className="mb-2 flex items-center justify-between">
          <strong className="text-xs text-sky-200">Provenance: {provenanceTarget}</strong>
          <button
            className="text-xs text-gray-400 hover:text-white"
            onClick={() => setProvenanceTarget("")}
          >
            Close
          </button>
        </div>
        {provenance.length === 0 ? (
          <p className="text-xs text-gray-400">No recorded provenance events.</p>
        ) : (
          <ol className="space-y-2">
            {provenance.map((event) => (
              <li key={event.id} className="border-l border-sky-700 pl-2 text-xs">
                <span className="text-gray-200">{event.eventKind}</span>
                <span className="ml-2 text-gray-500">{formatInstant(event.occurredAtUs)}</span>
                {event.actorId && <span className="ml-2 text-gray-400">by {event.actorId}</span>}
                {event.reason && <p className="mt-1 text-gray-400">{event.reason}</p>}
              </li>
            ))}
          </ol>
        )}
      </div>
    ) : null;

  const toggleClaim = (
    values: string[],
    setter: React.Dispatch<React.SetStateAction<string[]>>,
    id: string,
  ) => setter(values.includes(id) ? values.filter((value) => value !== id) : [...values, id]);

  return (
    <section className="mt-6 rounded-2xl border border-[#263044] bg-[#10141b] p-5">
      <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-sm font-bold text-gray-100">Versioned knowledge acquisition</h2>
          <p className="mt-1 max-w-3xl text-xs text-gray-400">
            Legacy source records remain unchanged. Versioned citations and extractions below
            are separate, explicitly linked records.
          </p>
        </div>
        <div className="flex gap-1 rounded-lg border border-[#2b3343] bg-[#151a23] p-1">
          {(["versions", "knowledge", "frameworks"] as const).map((value) => (
            <button
              key={value}
              onClick={() => setTab(value)}
              className={`rounded px-2.5 py-1 text-xs capitalize ${
                tab === value ? "bg-amber-500 text-black" : "text-gray-400 hover:text-white"
              }`}
            >
              {value}
            </button>
          ))}
        </div>
      </div>

      {error && (
        <div role="alert" className="mb-3 rounded border border-rose-800 bg-rose-950/40 p-2 text-xs text-rose-200">
          {error}
        </div>
      )}
      {notice && <p role="status" className="mb-3 text-xs text-emerald-300">{notice}</p>}

      {tab === "versions" && (
        <div className="space-y-5">
          <form onSubmit={addVersion} className="grid gap-2 rounded-lg border border-[#283143] p-3 md:grid-cols-4">
            <input
              value={versionLabel}
              onChange={(event) => setVersionLabel(event.target.value)}
              placeholder="Edition or capture label"
              className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs"
            />
            <input
              value={checksum}
              onChange={(event) => setChecksum(event.target.value)}
              placeholder="SHA-256 (optional)"
              className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs"
            />
            <input
              value={assetRef}
              onChange={(event) => setAssetRef(event.target.value)}
              placeholder="External asset reference (optional)"
              className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs"
            />
            <button disabled={busy} className="rounded bg-amber-500 px-3 py-1.5 text-xs font-semibold text-black disabled:opacity-50">
              Capture source version
            </button>
          </form>

          <div className="grid gap-4 lg:grid-cols-[minmax(220px,0.8fr)_minmax(0,2fr)]">
            <div className="space-y-2">
              <h3 className="text-xs font-semibold text-gray-200">Immutable source versions</h3>
              {versions.length === 0 ? (
                <p className="rounded border border-dashed border-[#384154] p-3 text-xs text-gray-500">
                  No version has been captured. Legacy source history is not treated as a version.
                </p>
              ) : (
                versions.map((version) => (
                  <button
                    key={version.id}
                    onClick={() => setVersionId(version.id)}
                    className={`block w-full rounded border p-3 text-left ${
                      version.id === versionId
                        ? "border-amber-500/60 bg-amber-500/10"
                        : "border-[#293244] bg-[#141923] hover:border-[#536078]"
                    }`}
                  >
                    <span className="block text-xs font-semibold text-gray-100">
                      {version.editionLabel || version.captureKind}
                    </span>
                    <span className="mt-1 block break-all font-mono text-[10px] text-gray-500">
                      {version.id}
                    </span>
                    <span className="mt-1 block text-[10px] text-gray-400">
                      Captured {formatInstant(version.capturedAtUs)}
                    </span>
                    {version.contentSha256 && (
                      <span className="mt-1 block break-all font-mono text-[10px] text-gray-400">
                        SHA-256 {version.contentSha256}
                      </span>
                    )}
                    <span className="mt-1 block text-[10px] text-sky-300">
                      Provenance: {version.createdBy}
                    </span>
                  </button>
                ))
              )}
            </div>

            {selectedVersion && (
              <div className="space-y-4">
                <div className="rounded-lg border border-[#293244] bg-[#141923] p-3">
                  <div className="flex flex-wrap items-start justify-between gap-2">
                    <div>
                      <h3 className="text-xs font-semibold text-gray-100">Version-pinned citations</h3>
                      <p className="mt-1 text-[10px] text-gray-500">
                        Each locator belongs to version {selectedVersion.id}.
                      </p>
                    </div>
                    <button
                      onClick={() => void showProvenance("source_version", selectedVersion.id)}
                      className="text-[10px] text-sky-300 hover:text-sky-200"
                    >
                      Provenance
                    </button>
                  </div>
                  <form onSubmit={addLocation} className="mt-3 grid gap-2 md:grid-cols-3">
                    <input
                      required
                      value={locationLabel}
                      onChange={(event) => setLocationLabel(event.target.value)}
                      placeholder="Citation label"
                      className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs"
                    />
                    <select
                      value={locationKind}
                      onChange={(event) => setLocationKind(event.target.value as SourceLocationKind)}
                      className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs"
                    >
                      {locationKinds.map((kind) => <option key={kind} value={kind}>{kind.replace("_", " ")}</option>)}
                    </select>
                    <select
                      value={legacySegmentId}
                      onChange={(event) => setLegacySegmentId(event.target.value)}
                      className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs"
                    >
                      <option value="">No legacy segment mapping</option>
                      {flatSegments.map((segment) => <option key={segment.id} value={segment.id}>{segment.label}</option>)}
                    </select>
                    {locationKind !== "text_anchor" && locationKind !== "unknown" && (
                      <>
                        <input
                          value={locationStart}
                          onChange={(event) => setLocationStart(event.target.value)}
                          placeholder={locationKind === "timestamp" || locationKind === "time_range" ? "Start (ms)" : "Start"}
                          type="number"
                          className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs"
                        />
                        {(locationKind === "page" || locationKind === "line" || locationKind === "time_range") && (
                          <input
                            value={locationEnd}
                            onChange={(event) => setLocationEnd(event.target.value)}
                            placeholder={locationKind === "time_range" ? "End (ms)" : "End"}
                            type="number"
                            className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs"
                          />
                        )}
                      </>
                    )}
                    {locationKind === "text_anchor" && (
                      <input
                        required
                        value={locationSelector}
                        onChange={(event) => setLocationSelector(event.target.value)}
                        placeholder="Stable selector"
                        className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs"
                      />
                    )}
                    <input
                      value={locationText}
                      onChange={(event) => setLocationText(event.target.value)}
                      placeholder="Locator text or quotation"
                      className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs md:col-span-2"
                    />
                    <button disabled={busy} className="rounded bg-[#263146] px-3 py-1.5 text-xs text-white disabled:opacity-50">
                      Save citation
                    </button>
                  </form>
                  <ul className="mt-3 space-y-2">
                    {locations.map((location) => {
                      const url = citedUrl(source, location);
                      return (
                        <li key={location.id} className="rounded border border-[#283143] p-2">
                          <div className="flex flex-wrap items-center justify-between gap-2">
                            <span className="text-xs font-medium text-gray-200">
                              {location.label} <span className="ml-1 text-gray-400">({locationDescription(location)})</span>
                            </span>
                            <div className="flex gap-3">
                              {url && <a href={url} target="_blank" rel="noreferrer" className="text-[10px] text-sky-300">Open at citation</a>}
                              <button onClick={() => setSelectedLocationId(location.id)} className="text-[10px] text-amber-300">Use for extraction</button>
                              <button onClick={() => void showProvenance("source_location", location.id)} className="text-[10px] text-sky-300">Provenance</button>
                            </div>
                          </div>
                          <p className="mt-1 break-all font-mono text-[10px] text-gray-500">
                            {location.sourceVersionId} / {location.id}
                            {location.legacySegmentId ? ` · linked legacy segment ${location.legacySegmentId}` : ""}
                          </p>
                          {location.locatorText && <p className="mt-1 whitespace-pre-wrap text-xs text-gray-400">{location.locatorText}</p>}
                        </li>
                      );
                    })}
                  </ul>
                </div>

                <div className="rounded-lg border border-[#293244] bg-[#141923] p-3">
                  <h3 className="text-xs font-semibold text-gray-200">Create an exact-version extraction</h3>
                  {!selectedLocationId && <p className="mt-1 text-[10px] text-amber-300">Choose a citation location first.</p>}
                  <form onSubmit={addExtraction} className="mt-3 space-y-2">
                    <div className="flex flex-wrap gap-2">
                      <select
                        value={extractionKind}
                        onChange={(event) => setExtractionKind(event.target.value as ExtractionKind)}
                        className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs"
                      >
                        {extractionKinds.map((kind) => <option key={kind} value={kind}>{kind.replace("_", " ")}</option>)}
                      </select>
                      <input value={toolIdentity} onChange={(event) => setToolIdentity(event.target.value)} placeholder="Tool/model identity (optional)" className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" />
                      <input value={toolVersion} onChange={(event) => setToolVersion(event.target.value)} placeholder="Tool version" className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" />
                      <input value={promptIdentity} onChange={(event) => setPromptIdentity(event.target.value)} placeholder="Prompt/config identity" className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" />
                    </div>
                    <textarea required value={extractionText} onChange={(event) => setExtractionText(event.target.value)} rows={3} placeholder="Extraction content" className="w-full rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" />
                    <button disabled={busy || !selectedLocationId} className="rounded bg-amber-500 px-3 py-1.5 text-xs font-semibold text-black disabled:opacity-50">
                      Save as unreviewed
                    </button>
                    <p className="text-[10px] text-amber-300">Every new extraction starts unreviewed, including machine output.</p>
                  </form>

                  <ul className="mt-3 space-y-2">
                    {extractions.map((extraction) => (
                      <li key={extraction.id} className="rounded border border-[#283143] p-3">
                        <div className="flex flex-wrap items-center justify-between gap-2">
                          <span className="text-xs font-semibold text-gray-200">
                            {extraction.kind.replace("_", " ")}
                            <span className={`ml-2 rounded px-1.5 py-0.5 text-[10px] ${extraction.reviewStatus === "unreviewed" ? "bg-amber-900/50 text-amber-200" : "bg-emerald-900/40 text-emerald-200"}`}>
                              {extraction.reviewStatus.replace("_", " ")}
                            </span>
                          </span>
                          <div className="flex flex-wrap gap-2">
                            {extraction.reviewStatus === "unreviewed" && (
                              <>
                                <button onClick={() => reviewExtraction(extraction.id, "accepted")} className="text-[10px] text-emerald-300">Accept quality</button>
                                <button onClick={() => reviewExtraction(extraction.id, "rejected")} className="text-[10px] text-rose-300">Reject</button>
                                <button onClick={() => reviewExtraction(extraction.id, "needs_correction")} className="text-[10px] text-amber-300">Needs correction</button>
                              </>
                            )}
                            {extraction.kind === "source_claim" && extraction.reviewStatus === "accepted" && !claims.some((claim) => claim.extractionId === extraction.id) && (
                              <button onClick={() => { setSelectedExtractionId(extraction.id); setClaimDraft(extraction.content); }} className="text-[10px] text-sky-300">Create attributed claim</button>
                            )}
                            <button onClick={() => { setSelectedExtractionId(extraction.id); setCorrectionText(extraction.content); }} className="text-[10px] text-gray-300">Correct</button>
                            <button onClick={() => void showProvenance("extraction", extraction.id)} className="text-[10px] text-sky-300">Provenance</button>
                          </div>
                        </div>
                        <p className="mt-2 whitespace-pre-wrap text-xs text-gray-300">{extraction.content}</p>
                        <p className="mt-2 break-all font-mono text-[10px] text-gray-500">
                          Source {extraction.sourceId} · Version {extraction.sourceVersionId} · Location {extraction.sourceSegmentId}
                        </p>
                        <p className="mt-1 text-[10px] text-gray-500">
                          {extraction.createdBy} · {formatInstant(extraction.createdAtUs)}
                          {extraction.toolVersion ? ` · tool ${extraction.toolVersion}` : ""}
                          {extraction.parentExtractionId ? ` · parent ${extraction.parentExtractionId}` : ""}
                          {extraction.supersededById ? ` · superseded by ${extraction.supersededById}` : ""}
                        </p>
                        {selectedExtractionId === extraction.id && (
                          <form onSubmit={correctExtraction} className="mt-3 grid gap-2">
                            <textarea required value={correctionText} onChange={(event) => setCorrectionText(event.target.value)} rows={2} className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" aria-label="Corrected extraction" />
                            <input required value={correctionReason} onChange={(event) => setCorrectionReason(event.target.value)} placeholder="Correction reason" className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" />
                            <button disabled={busy} className="justify-self-start rounded bg-[#263146] px-3 py-1.5 text-xs">Save correction as new record</button>
                          </form>
                        )}
                        {selectedExtractionId === extraction.id && extraction.kind === "source_claim" && extraction.reviewStatus === "accepted" && !claims.some((claim) => claim.extractionId === extraction.id) && (
                          <div className="mt-3 flex gap-2">
                            <textarea value={claimDraft} onChange={(event) => setClaimDraft(event.target.value)} rows={2} className="min-w-0 flex-1 rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" aria-label="Source claim statement" />
                            <button disabled={busy} onClick={() => createClaim(extraction)} className="self-end rounded bg-sky-700 px-3 py-1.5 text-xs">Save source claim</button>
                          </div>
                        )}
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {tab === "knowledge" && (
        <div className="grid gap-4 lg:grid-cols-2">
          <div className="space-y-3">
            <h3 className="text-xs font-semibold text-gray-200">Source-attributed claims</h3>
            {claims.length === 0 ? (
              <p className="text-xs text-gray-500">No source claims yet. Create one from an accepted source-claim extraction.</p>
            ) : claims.map((claim) => (
              <article key={claim.id} className={`rounded-lg border p-3 ${claim.id === selectedClaimId ? "border-amber-500/60 bg-amber-500/5" : "border-[#293244] bg-[#141923]"}`}>
                <button onClick={() => setSelectedClaimId(claim.id)} className="text-left text-xs font-semibold text-gray-100">{claim.statement}</button>
                <p className="mt-1 break-all font-mono text-[10px] text-gray-500">
                  {claim.id} · source {claim.sourceId} · version {claim.sourceVersionId} · extraction {claim.extractionId}
                </p>
                <p className="mt-1 text-[10px] text-amber-300">Source assertion; not an application-verified scientific fact.</p>
                <button onClick={() => void showProvenance("source_claim", claim.id)} className="mt-2 text-[10px] text-sky-300">Provenance</button>
              </article>
            ))}

            {selectedClaim && (
              <div className="space-y-3 rounded-lg border border-[#293244] bg-[#141923] p-3">
                <form onSubmit={createKnowledgeItem} className="space-y-2">
                  <h4 className="text-xs font-semibold text-gray-200">Create sourced knowledge item</h4>
                  <select value={knowledgeKind} onChange={(event) => setKnowledgeKind(event.target.value as typeof knowledgeKind)} className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs">
                    {itemKinds.map((kind) => <option key={kind} value={kind}>{kind}</option>)}
                  </select>
                  <input required value={knowledgeTitle} onChange={(event) => setKnowledgeTitle(event.target.value)} placeholder="Title" className="w-full rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" />
                  <textarea required value={knowledgeContent} onChange={(event) => setKnowledgeContent(event.target.value)} rows={3} placeholder="Definition, principle, or procedure" className="w-full rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" />
                  <button disabled={busy} className="rounded bg-amber-500 px-3 py-1.5 text-xs font-semibold text-black">Save proposed item</button>
                </form>

                <form onSubmit={addClaimRelation} className="space-y-2 border-t border-[#2b3343] pt-3">
                  <h4 className="text-xs font-semibold text-gray-200">Preserve claim relationships</h4>
                  <select value={relationTargetId} onChange={(event) => setRelationTargetId(event.target.value)} className="w-full rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs">
                    <option value="">Choose another source claim</option>
                    {claims.filter((claim) => claim.id !== selectedClaim.id).map((claim) => <option key={claim.id} value={claim.id}>{claim.statement}</option>)}
                  </select>
                  <div className="flex gap-2">
                    <select value={relationKind} onChange={(event) => setRelationKind(event.target.value as ClaimRelationKind)} className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs">
                      {(["supports", "contradicts", "qualifies"] as const).map((kind) => <option key={kind} value={kind}>{kind}</option>)}
                    </select>
                    <button disabled={busy || !relationTargetId} className="rounded bg-[#263146] px-3 py-1.5 text-xs">Record relationship</button>
                  </div>
                  <ul className="space-y-1">
                    {relations.map((relation) => (
                      <li key={`${claimRelationTargetKind(relation)}:${relation.targetId}:${relation.relationKind}`} className="text-[10px] text-gray-400">
                        {relation.relationKind} → {relation.targetKind} {relation.targetId}
                      </li>
                    ))}
                  </ul>
                </form>
              </div>
            )}
          </div>

          <div className="space-y-3">
            <h3 className="text-xs font-semibold text-gray-200">Concepts, principles, and procedures</h3>
            <ul className="space-y-2">
              {items.map((item) => (
                <li key={`${item.kind}:${item.id}`} className="rounded-lg border border-[#293244] bg-[#141923] p-3">
                  <div className="flex items-start justify-between gap-2">
                    <div>
                      <span className="text-[10px] uppercase text-sky-300">{item.kind} · {item.lifecycle}</span>
                      <h4 className="mt-1 text-xs font-semibold text-gray-100">{item.title}</h4>
                    </div>
                    <button onClick={() => { setSelectedItemId(item.id); void showProvenance(item.kind, item.id); }} className="text-[10px] text-sky-300">Provenance</button>
                  </div>
                  <p className="mt-2 whitespace-pre-wrap text-xs text-gray-400">{item.content}</p>
                  {item.sourceClaimId && <p className="mt-2 text-[10px] text-gray-500">Attributed to source claim {item.sourceClaimId} via {item.relationKind}.</p>}
                  {selectedItemId === item.id && <p className="mt-1 break-all font-mono text-[10px] text-gray-500">{item.id}</p>}
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}

      {tab === "frameworks" && (
        <div className="space-y-4">
          <div className="grid gap-4 lg:grid-cols-[minmax(220px,0.8fr)_minmax(0,2fr)]">
            <div className="space-y-3">
              <form onSubmit={createFramework} className="space-y-2 rounded-lg border border-[#293244] bg-[#141923] p-3">
                <h3 className="text-xs font-semibold text-gray-200">Frameworks</h3>
                <input required value={frameworkTitle} onChange={(event) => setFrameworkTitle(event.target.value)} placeholder="New framework title" className="w-full rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" />
                <button disabled={busy} className="rounded bg-amber-500 px-3 py-1.5 text-xs font-semibold text-black">Create draft</button>
              </form>
              <div className="space-y-1">
                {frameworks.map((framework) => (
                  <button key={framework.id} onClick={() => setFrameworkId(framework.id)} className={`block w-full rounded border p-2 text-left ${frameworkId === framework.id ? "border-amber-500/60 bg-amber-500/5" : "border-[#293244] bg-[#141923]"}`}>
                    <span className="block text-xs text-gray-100">{framework.title}</span>
                    <span className="text-[10px] text-gray-500">{framework.lifecycle} · {framework.id}</span>
                  </button>
                ))}
              </div>
              {selectedFramework && (
                <div className="space-y-2 rounded-lg border border-[#293244] bg-[#141923] p-3">
                  <h4 className="text-xs font-semibold text-gray-200">Publication history</h4>
                  {frameworkVersions.map((version) => (
                    <button key={version.id} onClick={() => setFrameworkVersionId(version.id)} className={`block w-full rounded p-2 text-left ${version.id === frameworkVersionId ? "bg-amber-500/10" : "hover:bg-[#1b2230]"}`}>
                      <span className="text-xs text-gray-200">v{version.versionNo} · {version.status}</span>
                      <span className="block text-[10px] text-gray-500">{formatInstant(version.createdAtUs)}{version.publishedAtUs ? ` · published ${formatInstant(version.publishedAtUs)}` : ""}</span>
                      {version.changeClass && <span className="block text-[10px] text-sky-300">{version.changeClass}: {version.changeRationale}</span>}
                    </button>
                  ))}
                </div>
              )}
            </div>

            {selectedFramework && selectedFrameworkVersion && (
              <div className="space-y-3">
                <div className="rounded-lg border border-[#293244] bg-[#141923] p-3">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div>
                      <h3 className="text-xs font-semibold text-gray-100">{selectedFramework.title} · v{selectedFrameworkVersion.versionNo}</h3>
                      <p className="mt-1 break-all font-mono text-[10px] text-gray-500">{selectedFrameworkVersion.id} · {selectedFrameworkVersion.status}</p>
                      <p className="mt-1 text-[10px] text-amber-300">Only drafts are editable. Published framework versions are immutable.</p>
                    </div>
                    <button onClick={() => void showProvenance("framework_version", selectedFrameworkVersion.id)} className="text-[10px] text-sky-300">Publication provenance</button>
                  </div>

                  {selectedFrameworkVersion.status === "draft" && (
                    <form onSubmit={updateFrameworkDraft} className="mt-3 grid gap-2">
                      <input value={frameworkDraftTitle} onChange={(event) => setFrameworkDraftTitle(event.target.value)} className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" aria-label="Framework draft title" />
                      <textarea value={frameworkDraftDescription} onChange={(event) => setFrameworkDraftDescription(event.target.value)} rows={2} className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" aria-label="Framework draft description" />
                      <button disabled={busy} className="justify-self-start rounded bg-[#263146] px-3 py-1.5 text-xs">Save draft metadata</button>
                    </form>
                  )}

                  {selectedFrameworkVersion.status === "draft" && (
                    <form onSubmit={addFrameworkItem} className="mt-3 flex flex-wrap gap-2 border-t border-[#2b3343] pt-3">
                      <select value={frameworkItemKind} onChange={(event) => { setFrameworkItemKind(event.target.value as KnowledgeItemKind); setFrameworkItemId(""); }} className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs">
                        {(["source_claim", ...itemKinds] as KnowledgeItemKind[]).map((kind) => <option key={kind} value={kind}>{kind}</option>)}
                      </select>
                      <select value={frameworkItemId} onChange={(event) => setFrameworkItemId(event.target.value)} className="min-w-48 flex-1 rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs">
                        <option value="">Choose version member</option>
                        {frameworkItemKind === "source_claim"
                          ? claims.map((claim) => <option key={claim.id} value={claim.id}>{claim.statement}</option>)
                          : items.filter((item) => item.kind === frameworkItemKind).map((item) => <option key={item.id} value={item.id}>{item.title}</option>)}
                      </select>
                      <select value={frameworkRole} onChange={(event) => setFrameworkRole(event.target.value as FrameworkMembershipRole)} className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs">
                        {membershipRoles.map((role) => <option key={role} value={role}>{role}</option>)}
                      </select>
                      <button disabled={busy || !frameworkItemId} className="rounded bg-[#263146] px-3 py-1.5 text-xs">Add ordered member</button>
                    </form>
                  )}

                  <ol className="mt-3 space-y-2">
                    {selectedFrameworkVersion.items.map((item) => (
                      <li key={item.id} className="flex items-start justify-between gap-2 rounded border border-[#283143] p-2">
                        <div>
                          <span className="text-[10px] text-sky-300">{item.itemOrder + 1}. {item.itemKind} · {item.membershipRole}</span>
                          <p className="text-xs text-gray-200">{item.itemTitle}</p>
                          <p className="break-all font-mono text-[10px] text-gray-500">{item.itemId}</p>
                        </div>
                        {selectedFrameworkVersion.status === "draft" && (
                          <button disabled={busy} onClick={() => void runAction(() => srcRemoveFrameworkItem(item.id), "Framework member removed from draft.")} className="text-[10px] text-rose-300">Remove</button>
                        )}
                      </li>
                    ))}
                  </ol>

                  {(selectedFrameworkVersion.status === "draft" || selectedFrameworkVersion.status === "review") && (
                    <div className="mt-4 grid gap-3 border-t border-[#2b3343] pt-3 md:grid-cols-2">
                      <form onSubmit={(event) => { event.preventDefault(); void runAction(async () => {
                        await srcReviewFrameworkVersion({
                          frameworkVersionId: selectedFrameworkVersion.id,
                          reviewStatus,
                          reviewerId: "user",
                          rationale: reviewRationale.trim() || null,
                        });
                        setReviewRationale("");
                      }, `Framework review recorded: ${reviewStatus}.`); }} className="space-y-2">
                        <h4 className="text-xs font-semibold text-gray-200">Framework review</h4>
                        <select value={reviewStatus} onChange={(event) => setReviewStatus(event.target.value as FrameworkReviewStatus)} className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs">
                          {(["approved", "changes_requested", "rejected"] as const).map((status) => <option key={status} value={status}>{status.replace("_", " ")}</option>)}
                        </select>
                        <input value={reviewRationale} onChange={(event) => setReviewRationale(event.target.value)} placeholder="Review rationale (optional)" className="w-full rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" />
                        <button disabled={busy} className="rounded bg-[#263146] px-3 py-1.5 text-xs">Record review</button>
                      </form>
                      <div className="space-y-2">
                        <h4 className="text-xs font-semibold text-gray-200">Publication</h4>
                        <p className="text-[10px] text-gray-400">Publication requires an approved latest review and at least one member. Publishing seals this snapshot.</p>
                        <button disabled={busy} onClick={() => void runAction(() => srcPublishFrameworkVersion(selectedFrameworkVersion.id), "Framework version published and sealed.")} className="rounded bg-emerald-700 px-3 py-1.5 text-xs disabled:opacity-50">Publish immutable version</button>
                      </div>
                    </div>
                  )}
                </div>

                {canCreateSuccessor && latestVersion && (
                  <form onSubmit={createNextFrameworkVersion} className="space-y-2 rounded-lg border border-sky-900/60 bg-sky-950/20 p-3">
                    <h3 className="text-xs font-semibold text-sky-100">Create a new version from published v{latestVersion.versionNo}</h3>
                    <div className="flex flex-wrap gap-2">
                      <select value={changeClass} onChange={(event) => setChangeClass(event.target.value as FrameworkChangeClass)} className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs">
                        {changeClasses.map((value) => <option key={value} value={value}>{value}</option>)}
                      </select>
                      <input required value={changeRationale} onChange={(event) => setChangeRationale(event.target.value)} placeholder="Change rationale" className="min-w-48 flex-1 rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs" />
                    </div>
                    <div className="grid gap-2 md:grid-cols-2">
                      <fieldset className="space-y-1">
                        <legend className="text-[10px] text-gray-300">Affected claims</legend>
                        {claims.map((claim) => <label key={`affected:${claim.id}`} className="flex gap-2 text-[10px] text-gray-400"><input type="checkbox" checked={affectedClaimIds.includes(claim.id)} onChange={() => toggleClaim(affectedClaimIds, setAffectedClaimIds, claim.id)} />{claim.statement}</label>)}
                      </fieldset>
                      <fieldset className="space-y-1">
                        <legend className="text-[10px] text-gray-300">Assumption links</legend>
                        {claims.map((claim) => <label key={`assumption:${claim.id}`} className="flex gap-2 text-[10px] text-gray-400"><input type="checkbox" checked={assumptionClaimIds.includes(claim.id)} onChange={() => toggleClaim(assumptionClaimIds, setAssumptionClaimIds, claim.id)} />{claim.statement}</label>)}
                      </fieldset>
                    </div>
                    <button disabled={busy} className="rounded bg-sky-700 px-3 py-1.5 text-xs">Create successor draft</button>
                  </form>
                )}

                <div className="rounded-lg border border-[#293244] bg-[#141923] p-3">
                  <h3 className="text-xs font-semibold text-gray-200">Version comparison</h3>
                  <div className="mt-2 flex flex-wrap items-center gap-2 text-xs">
                    <span className="text-gray-400">Compare selected version with</span>
                    <select value={compareVersionId} onChange={(event) => setCompareVersionId(event.target.value)} className="rounded border border-[#394153] bg-[#0c1016] px-2 py-1.5 text-xs">
                      {frameworkVersions.filter((version) => version.id !== selectedFrameworkVersion.id).map((version) => <option key={version.id} value={version.id}>v{version.versionNo} · {version.status}</option>)}
                    </select>
                  </div>
                  {!comparedVersion || comparedVersion.id === selectedFrameworkVersion.id ? (
                    <p className="mt-2 text-xs text-gray-500">Select another framework version to compare membership changes.</p>
                  ) : (
                    <div className="mt-3 grid gap-3 md:grid-cols-2">
                      <div>
                        <h4 className="text-[10px] font-semibold text-emerald-300">Added in v{selectedFrameworkVersion.versionNo}</h4>
                        {addedItems.length ? addedItems.map((item) => <p key={item.id} className="mt-1 text-xs text-gray-300">+ {item.itemKind}: {item.itemTitle}</p>) : <p className="mt-1 text-xs text-gray-500">No added members.</p>}
                      </div>
                      <div>
                        <h4 className="text-[10px] font-semibold text-rose-300">Absent from v{selectedFrameworkVersion.versionNo}</h4>
                        {removedItems.length ? removedItems.map((item) => <p key={item.id} className="mt-1 text-xs text-gray-300">− {item.itemKind}: {item.itemTitle}</p>) : <p className="mt-1 text-xs text-gray-500">No removed members.</p>}
                      </div>
                      <div className="md:col-span-2">
                        <h4 className="text-[10px] font-semibold text-amber-300">Reordered or reclassified members</h4>
                        {changedItems.length ? changedItems.map((item) => {
                          const previous = oldMemberships.get(membershipKey(item));
                          return <p key={item.id} className="mt-1 text-xs text-gray-300">~ {item.itemTitle}: {previous?.membershipRole} / order {previous?.itemOrder} → {item.membershipRole} / order {item.itemOrder}</p>;
                        }) : <p className="mt-1 text-xs text-gray-500">No membership role or order changes.</p>}
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {renderProvenance()}
      {busy && <p className="mt-3 text-[10px] text-gray-500">Saving…</p>}
    </section>
  );
};
