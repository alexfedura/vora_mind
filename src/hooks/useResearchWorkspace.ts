import { useCallback, useEffect, useState } from "react";
import { researchService } from "@/mind/services/researchService";
import type { ResearchWorkspaceDto } from "@/types";

export function useResearchWorkspace() {
  const [workspace, setWorkspace] = useState<ResearchWorkspaceDto | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async (): Promise<void> => {
    setIsLoading(true);
    setError(null);
    try {
      setWorkspace(await researchService.getWorkspace());
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      setError(message);
      throw cause;
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh().catch((cause: unknown) => {
      console.error("Could not load Research workspace:", cause);
    });
  }, [refresh]);

  const run = useCallback(async <T,>(action: () => Promise<T>): Promise<T> => {
    setError(null);
    try {
      const result = await action();
      await refresh();
      return result;
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      setError(message);
      throw cause;
    }
  }, [refresh]);

  return { workspace, isLoading, error, refresh, run };
}
