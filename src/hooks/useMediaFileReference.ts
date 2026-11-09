import { useEffect, useState } from "react";
import { dbService } from "@/mind/services/dbService";

interface MediaFileReferenceState {
  fileId: string | null;
  filePath: string;
  isAvailable: boolean;
  noteCount: number;
  error: string | null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function useMediaFileReference(
  nodeId: string,
  initialFileId: string | undefined,
  fallbackPath: string,
): MediaFileReferenceState {
  const [reference, setReference] = useState<MediaFileReferenceState>({
    fileId: initialFileId ?? null,
    filePath: fallbackPath,
    isAvailable: true,
    noteCount: 0,
    error: null,
  });

  useEffect(() => {
    let cancelled = false;
    let fileId = initialFileId ?? null;

    const refresh = async (): Promise<void> => {
      try {
        if (!fileId) {
          fileId = await dbService.importFileReference(fallbackPath);
          if (cancelled) return;
          window.dispatchEvent(new CustomEvent("vora:attach-media-file-reference", {
            detail: { nodeId, fileId },
          }));
        }
        const [file, slices] = await Promise.all([
          dbService.getVoraFile(fileId),
          dbService.listMediaSlices(fileId),
        ]);
        if (cancelled) return;
        setReference({
          fileId,
          filePath: file.current_path,
          isAvailable: file.is_available,
          noteCount: slices.length,
          error: null,
        });
      } catch (cause: unknown) {
        if (cancelled) return;
        console.error("Could not load the canvas media reference:", cause);
        setReference((current) => ({
          ...current,
          error: cause instanceof Error ? cause.message : String(cause),
        }));
      }
    };

    const shouldRefresh = (event: Event): boolean => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return true;
      return typeof event.detail.fileId !== "string" || event.detail.fileId === fileId;
    };
    const handleReferenceUpdate = (event: Event): void => {
      if (shouldRefresh(event)) void refresh();
    };
    window.addEventListener("vora:media-slices-changed", handleReferenceUpdate);
    window.addEventListener("vora:file-reference-updated", handleReferenceUpdate);
    void refresh();

    return () => {
      cancelled = true;
      window.removeEventListener("vora:media-slices-changed", handleReferenceUpdate);
      window.removeEventListener("vora:file-reference-updated", handleReferenceUpdate);
    };
  }, [fallbackPath, initialFileId, nodeId]);

  return reference;
}
