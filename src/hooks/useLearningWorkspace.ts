import { useCallback, useEffect, useState } from "react";
import { learningService } from "@/mind/services/learningService";
import { LatestRequestGuard } from "@/shared/layout/researchAsyncGuards";
import type { LearningWorkspaceDto } from "@/types";

export function useLearningWorkspace() {
  const [workspace, setWorkspace] = useState<LearningWorkspaceDto | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [requestGuard] = useState(() => new LatestRequestGuard());

  const refresh = useCallback(async (): Promise<void> => {
    const requestId = requestGuard.begin();
    if (!requestGuard.isCurrent(requestId)) return;
    setIsLoading(true);
    setError(null);
    try {
      const nextWorkspace = await learningService.getWorkspace();
      requestGuard.commit(requestId, () => setWorkspace(nextWorkspace));
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      requestGuard.commit(requestId, () => setError(message));
    } finally {
      requestGuard.commit(requestId, () => setIsLoading(false));
    }
  }, [requestGuard]);

  useEffect(() => {
    requestGuard.activate();
    void refresh();
    return () => requestGuard.deactivate();
  }, [refresh, requestGuard]);

  const run = useCallback(async <T,>(
    action: () => Promise<T>,
    isActionCurrent: () => boolean = () => true,
  ): Promise<T> => {
    const requestId = requestGuard.begin();
    requestGuard.commit(requestId, () => setError(null));
    try {
      const result = await action();
      if (!isActionCurrent()) return result;
      await refresh();
      return result;
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      if (isActionCurrent()) {
        requestGuard.commit(requestId, () => setError(message));
      }
      throw cause;
    }
  }, [refresh, requestGuard]);

  return { workspace, isLoading, error, refresh, run };
}
