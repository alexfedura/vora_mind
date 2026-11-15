import { useCallback, useEffect, useState } from "react";
import { researchService } from "@/mind/services/researchService";
import { LatestRequestGuard } from "@/shared/layout/researchAsyncGuards";
import type { ResearchWorkspaceDto } from "@/types";

export function useResearchWorkspace() {
  const [workspace, setWorkspace] = useState<ResearchWorkspaceDto | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [requestGuard] = useState(() => new LatestRequestGuard());

  const refresh = useCallback(async (): Promise<void> => {
    const requestId = requestGuard.begin();
    if (!requestGuard.isCurrent(requestId)) return;
    setIsLoading(true);
    setError(null);
    try {
      const nextWorkspace = await researchService.getWorkspace();
      requestGuard.commit(requestId, () => setWorkspace(nextWorkspace));
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      if (!requestGuard.commit(requestId, () => setError(message))) return;
      throw cause;
    } finally {
      requestGuard.commit(requestId, () => setIsLoading(false));
    }
  }, [requestGuard]);

  useEffect(() => {
    requestGuard.activate();
    void refresh().catch((cause: unknown) => {
      console.error("Could not load Research workspace:", cause);
    });
    return () => requestGuard.deactivate();
  }, [refresh, requestGuard]);

  const run = useCallback(async <T,>(
    action: () => Promise<T>,
    isActionCurrent: () => boolean = () => true,
  ): Promise<T> => {
    setError(null);
    try {
      const result = await action();
      await refresh();
      return result;
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      if (isActionCurrent()) setError(message);
      throw cause;
    }
  }, [refresh]);

  return { workspace, isLoading, error, refresh, run };
}
