import { useCallback, useEffect, useState } from "react";
import { analysisService } from "@/mind/services/analysisService";
import { experimentService } from "@/mind/services/experimentService";
import { knowledgeTransferService } from "@/mind/services/knowledgeTransferService";
import { modelService } from "@/mind/services/modelService";
import { LatestRequestGuard } from "@/shared/layout/researchAsyncGuards";
import type { ProjectLifecycleDomain } from "@/shared/layout/projectLifecycleModel";
import type {
  AnalysisWorkspaceDto,
  ExperimentWorkspaceDto,
  ModelWorkspaceDto,
  TransferWorkspaceDto,
} from "@/types";

export interface ProjectLifecycleSnapshot {
  analysis: ProjectLifecycleDomain<AnalysisWorkspaceDto>;
  experiment: ProjectLifecycleDomain<ExperimentWorkspaceDto>;
  model: ProjectLifecycleDomain<ModelWorkspaceDto>;
  transfer: ProjectLifecycleDomain<TransferWorkspaceDto>;
}

function domain<T>(
  status: ProjectLifecycleDomain<T>["status"],
  data: T | null = null,
  error: string | null = null,
): ProjectLifecycleDomain<T> {
  return { status, data, error };
}

function emptySnapshot(status: ProjectLifecycleDomain<never>["status"]): ProjectLifecycleSnapshot {
  return {
    analysis: domain(status),
    experiment: domain(status),
    model: domain(status),
    transfer: domain(status),
  };
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function loadDomain<T>(load: () => Promise<T>): Promise<ProjectLifecycleDomain<T>> {
  return Promise.resolve()
    .then(load)
    .then(
      (data) => domain<T>("loaded", data),
      (error: unknown) => domain<T>("error", null, messageOf(error)),
    );
}

export function useProjectLifecycleWorkbench(projectId: string) {
  const [snapshot, setSnapshot] = useState<ProjectLifecycleSnapshot>(() => emptySnapshot("idle"));
  const [refreshVersion, setRefreshVersion] = useState(0);
  const [guard] = useState(() => new LatestRequestGuard());

  useEffect(() => {
    guard.activate();
    return () => guard.deactivate();
  }, [guard]);

  useEffect(() => {
    if (!projectId) {
      setSnapshot(emptySnapshot("idle"));
      return;
    }

    const requestId = guard.begin();
    setSnapshot(emptySnapshot("loading"));
    void Promise.all([
      loadDomain(() => analysisService.getWorkspace(projectId)),
      loadDomain(() => experimentService.getWorkspace(projectId)),
      loadDomain(() => modelService.getWorkspace(projectId)),
      loadDomain(() => knowledgeTransferService.getWorkspace()),
    ]).then(([analysis, experiment, model, transfer]) => {
      guard.commit(requestId, () => {
        setSnapshot({ analysis, experiment, model, transfer });
      });
    });

    return () => {
      guard.begin();
    };
  }, [guard, projectId, refreshVersion]);

  const refresh = useCallback(() => {
    setRefreshVersion((current) => current + 1);
  }, []);

  return { ...snapshot, refresh };
}
