import { useCallback, useEffect, useState } from "react";
import { dbService } from "@/mind/services/dbService";
import type { MediaFileLibraryEntry } from "@/mind/types";

export function useMediaFiles(active: boolean) {
  const [files, setFiles] = useState<MediaFileLibraryEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [locatingFileId, setLocatingFileId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async (): Promise<void> => {
    setIsLoading(true);
    setError(null);
    try {
      setFiles(await dbService.listMediaFiles());
    } catch (cause: unknown) {
      console.error("Could not load Media & Files:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!active) return;
    const handleFileUpdate = (): void => {
      void refresh();
    };
    window.addEventListener("vora:file-reference-updated", handleFileUpdate);
    window.addEventListener("vora:media-slices-changed", handleFileUpdate);
    void refresh();
    return () => {
      window.removeEventListener("vora:file-reference-updated", handleFileUpdate);
      window.removeEventListener("vora:media-slices-changed", handleFileUpdate);
    };
  }, [active, refresh]);

  const locateFile = useCallback(async (fileId: string, searchDir: string): Promise<void> => {
    setLocatingFileId(fileId);
    setError(null);
    try {
      await dbService.resolveMissingFile(fileId, searchDir);
      await refresh();
      window.dispatchEvent(new CustomEvent("vora:file-reference-updated", {
        detail: { fileId },
      }));
    } catch (cause: unknown) {
      console.error("Could not locate file reference:", cause);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLocatingFileId(null);
    }
  }, [refresh]);

  return { files, isLoading, locatingFileId, error, refresh, locateFile };
}
