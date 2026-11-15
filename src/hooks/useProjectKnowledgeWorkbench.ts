import { useCallback, useEffect, useState } from "react";
import {
  srcListClaimRelations,
  srcListFrameworkVersions,
  srcListKnowledgeExtractions,
  srcListProvenance,
  srcListSourceClaims,
  srcListSourceLocations,
  srcListSourceVersions,
} from "@/source/services/sourceService";
import { LatestRequestGuard } from "@/shared/layout/researchAsyncGuards";
import type {
  AcquisitionProvenanceTargetKind,
  ClaimRelationDto,
  FrameworkVersionDto,
  KnowledgeExtractionDto,
  ProvenanceEventDto,
  SourceClaimDto,
  SourceLocationDto,
  SourceVersionDto,
} from "@/types";

export type ProjectKnowledgeLoadStatus = "idle" | "loading" | "loaded" | "error";

export interface ProjectKnowledgeSourceData {
  sourceId: string;
  status: ProjectKnowledgeLoadStatus;
  versions: SourceVersionDto[];
  claims: SourceClaimDto[];
  error: string | null;
}

export interface ProjectKnowledgeVersionData {
  versionId: string;
  status: ProjectKnowledgeLoadStatus;
  locations: SourceLocationDto[];
  extractions: KnowledgeExtractionDto[];
  error: string | null;
}

export interface ProjectKnowledgeRelationData {
  claimId: string;
  status: ProjectKnowledgeLoadStatus;
  relations: ClaimRelationDto[];
  error: string | null;
}

export interface ProjectKnowledgeProvenanceData {
  targetKind: AcquisitionProvenanceTargetKind;
  targetId: string;
  status: ProjectKnowledgeLoadStatus;
  events: ProvenanceEventDto[];
  error: string | null;
}

export interface ProjectKnowledgeFrameworkData {
  versionId: string;
  status: ProjectKnowledgeLoadStatus;
  version: FrameworkVersionDto | null;
  error: string | null;
}

interface UseProjectKnowledgeWorkbenchOptions {
  sourceId: string;
  sourceVersionId: string;
  sourceClaimId: string;
  frameworkId: string | null;
  frameworkVersionId: string | null;
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function useProjectKnowledgeWorkbench({
  sourceId,
  sourceVersionId,
  sourceClaimId,
  frameworkId,
  frameworkVersionId,
}: UseProjectKnowledgeWorkbenchOptions) {
  const [sourceRefresh, setSourceRefresh] = useState(0);
  const [sourceData, setSourceData] = useState<ProjectKnowledgeSourceData>({
    sourceId: "",
    status: "idle",
    versions: [],
    claims: [],
    error: null,
  });
  const [versionData, setVersionData] = useState<ProjectKnowledgeVersionData>({
    versionId: "",
    status: "idle",
    locations: [],
    extractions: [],
    error: null,
  });
  const [relationData, setRelationData] = useState<ProjectKnowledgeRelationData>({
    claimId: "",
    status: "idle",
    relations: [],
    error: null,
  });
  const [provenanceData, setProvenanceData] = useState<ProjectKnowledgeProvenanceData | null>(null);
  const [frameworkData, setFrameworkData] = useState<ProjectKnowledgeFrameworkData>({
    versionId: "",
    status: "idle",
    version: null,
    error: null,
  });
  const [sourceGuard] = useState(() => new LatestRequestGuard());
  const [versionGuard] = useState(() => new LatestRequestGuard());
  const [relationGuard] = useState(() => new LatestRequestGuard());
  const [provenanceGuard] = useState(() => new LatestRequestGuard());
  const [frameworkGuard] = useState(() => new LatestRequestGuard());

  const selectedVersion = sourceData.sourceId === sourceId
    ? sourceData.versions.find(
      (version) => version.id === sourceVersionId && version.sourceId === sourceId,
    ) ?? null
    : null;
  const selectedClaim = sourceData.sourceId === sourceId
    ? sourceData.claims.find(
      (claim) =>
        claim.id === sourceClaimId
        && claim.sourceId === sourceId
        && claim.sourceVersionId === sourceVersionId,
    ) ?? null
    : null;

  useEffect(() => {
    const guards = [sourceGuard, versionGuard, relationGuard, provenanceGuard, frameworkGuard];
    guards.forEach((guard) => guard.activate());
    return () => guards.forEach((guard) => guard.deactivate());
  }, [frameworkGuard, provenanceGuard, relationGuard, sourceGuard, versionGuard]);

  useEffect(() => {
    if (!sourceId) {
      const requestId = sourceGuard.begin();
      sourceGuard.commit(requestId, () => setSourceData({
        sourceId: "",
        status: "idle",
        versions: [],
        claims: [],
        error: null,
      }));
      return;
    }

    const requestId = sourceGuard.begin();
    setSourceData({
      sourceId,
      status: "loading",
      versions: [],
      claims: [],
      error: null,
    });
    void Promise.all([
      srcListSourceVersions(sourceId),
      srcListSourceClaims(sourceId),
    ]).then(([versions, claims]) => {
      sourceGuard.commit(requestId, () => setSourceData({
        sourceId,
        status: "loaded",
        versions,
        claims,
        error: null,
      }));
    }).catch((error: unknown) => {
      sourceGuard.commit(requestId, () => setSourceData({
        sourceId,
        status: "error",
        versions: [],
        claims: [],
        error: messageOf(error),
      }));
    });
  }, [sourceId, sourceGuard, sourceRefresh]);

  useEffect(() => {
    if (!selectedVersion) {
      const requestId = versionGuard.begin();
      versionGuard.commit(requestId, () => setVersionData({
        versionId: "",
        status: "idle",
        locations: [],
        extractions: [],
        error: null,
      }));
      return;
    }

    const versionId = selectedVersion.id;
    const requestId = versionGuard.begin();
    setVersionData({
      versionId,
      status: "loading",
      locations: [],
      extractions: [],
      error: null,
    });
    void Promise.all([
      srcListSourceLocations(versionId),
      srcListKnowledgeExtractions(versionId),
    ]).then(([locations, extractions]) => {
      versionGuard.commit(requestId, () => setVersionData({
        versionId,
        status: "loaded",
        locations,
        extractions,
        error: null,
      }));
    }).catch((error: unknown) => {
      versionGuard.commit(requestId, () => setVersionData({
        versionId,
        status: "error",
        locations: [],
        extractions: [],
        error: messageOf(error),
      }));
    });
  }, [selectedVersion?.id, versionGuard]);

  useEffect(() => {
    if (!selectedClaim) {
      const requestId = relationGuard.begin();
      relationGuard.commit(requestId, () => setRelationData({
        claimId: "",
        status: "idle",
        relations: [],
        error: null,
      }));
      return;
    }

    const claimId = selectedClaim.id;
    const requestId = relationGuard.begin();
    setRelationData({ claimId, status: "loading", relations: [], error: null });
    void srcListClaimRelations(claimId).then((relations) => {
      relationGuard.commit(requestId, () => setRelationData({
        claimId,
        status: "loaded",
        relations,
        error: null,
      }));
    }).catch((error: unknown) => {
      relationGuard.commit(requestId, () => setRelationData({
        claimId,
        status: "error",
        relations: [],
        error: messageOf(error),
      }));
    });
  }, [relationGuard, selectedClaim?.id]);

  useEffect(() => {
    if (!frameworkId || !frameworkVersionId) {
      const requestId = frameworkGuard.begin();
      frameworkGuard.commit(requestId, () => setFrameworkData({
        versionId: "",
        status: "idle",
        version: null,
        error: null,
      }));
      return;
    }

    const requestId = frameworkGuard.begin();
    setFrameworkData({
      versionId: frameworkVersionId,
      status: "loading",
      version: null,
      error: null,
    });
    void srcListFrameworkVersions(frameworkId).then((versions) => {
      const version = versions.find((candidate) => candidate.id === frameworkVersionId);
      frameworkGuard.commit(requestId, () => setFrameworkData({
        versionId: frameworkVersionId,
        status: version ? "loaded" : "error",
        version: version ?? null,
        error: version ? null : `Adopted FrameworkVersion ${frameworkVersionId} is unavailable.`,
      }));
    }).catch((error: unknown) => {
      frameworkGuard.commit(requestId, () => setFrameworkData({
        versionId: frameworkVersionId,
        status: "error",
        version: null,
        error: messageOf(error),
      }));
    });
  }, [frameworkGuard, frameworkId, frameworkVersionId]);

  const loadProvenance = useCallback(async (
    targetKind: AcquisitionProvenanceTargetKind,
    targetId: string,
  ): Promise<void> => {
    const requestId = provenanceGuard.begin();
    setProvenanceData({
      targetKind,
      targetId,
      status: "loading",
      events: [],
      error: null,
    });
    try {
      const events = await srcListProvenance(targetKind, targetId);
      provenanceGuard.commit(requestId, () => setProvenanceData({
        targetKind,
        targetId,
        status: "loaded",
        events,
        error: null,
      }));
    } catch (error: unknown) {
      provenanceGuard.commit(requestId, () => setProvenanceData({
        targetKind,
        targetId,
        status: "error",
        events: [],
        error: messageOf(error),
      }));
    }
  }, [provenanceGuard]);

  const refreshSource = useCallback(() => {
    setSourceRefresh((current) => current + 1);
  }, []);

  const closeProvenance = useCallback(() => {
    provenanceGuard.begin();
    setProvenanceData(null);
  }, [provenanceGuard]);

  return {
    sourceData,
    versionData,
    relationData,
    provenanceData,
    frameworkData,
    selectedVersion,
    selectedClaim,
    refreshSource,
    loadProvenance,
    closeProvenance,
  };
}
