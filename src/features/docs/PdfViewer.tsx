import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Document, Page, pdfjs } from "react-pdf";
import "react-pdf/dist/Page/AnnotationLayer.css";
import "react-pdf/dist/Page/TextLayer.css";
import { dbService, type DocumentDto, type DocumentHighlightDto } from "@/mind/services/dbService";
import { HighlightPopover, type HighlightColor, type PendingDocumentHighlight } from "./HighlightPopover";

pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  "pdfjs-dist/build/pdf.worker.min.mjs",
  import.meta.url,
).toString();

interface PdfRect {
  pageNumber: number;
  x: number;
  y: number;
  width: number;
  height: number;
}

interface PdfPositionData {
  rects: PdfRect[];
}

interface PdfViewerProps {
  document: DocumentDto;
  jumpToHighlightId: string | null;
  boardId: string | null;
  onSendToLibrary: (highlightId: string, content: string, boardId: string) => Promise<void>;
}

const highlightColors: Record<string, string> = {
  yellow: "rgba(250, 204, 21, .4)",
  green: "rgba(52, 211, 153, .36)",
  blue: "rgba(56, 189, 248, .36)",
  pink: "rgba(244, 114, 182, .36)",
};

function getPdfRects(positionData: unknown): PdfRect[] {
  if (typeof positionData !== "object" || positionData === null || !("rects" in positionData)) return [];
  const rects = positionData.rects;
  if (!Array.isArray(rects)) return [];
  return rects.filter((rect): rect is PdfRect => (
    typeof rect === "object" &&
    rect !== null &&
    "pageNumber" in rect && typeof rect.pageNumber === "number" &&
    "x" in rect && typeof rect.x === "number" &&
    "y" in rect && typeof rect.y === "number" &&
    "width" in rect && typeof rect.width === "number" &&
    "height" in rect && typeof rect.height === "number"
  ));
}

export const PdfViewer: React.FC<PdfViewerProps> = ({
  document,
  jumpToHighlightId,
  boardId,
  onSendToLibrary,
}) => {
  const [fileBytes, setFileBytes] = useState<Uint8Array | null>(null);
  const [pageCount, setPageCount] = useState(0);
  const [width, setWidth] = useState(850);
  const [highlights, setHighlights] = useState<DocumentHighlightDto[]>([]);
  const [selection, setSelection] = useState<PendingDocumentHighlight | null>(null);
  const [popoverPosition, setPopoverPosition] = useState<{ x: number; y: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const viewerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    setError(null);
    setFileBytes(null);
    void Promise.all([
      dbService.getDocumentContent(document.id),
      dbService.getHighlights(document.id),
    ]).then(([bytes, loadedHighlights]) => {
      if (cancelled) return;
      setFileBytes(bytes);
      setHighlights(loadedHighlights);
    }).catch((cause: unknown) => {
      if (!cancelled) {
        console.error("Could not load PDF document:", cause);
        setError(cause instanceof Error ? cause.message : String(cause));
      }
    });
    return () => { cancelled = true; };
  }, [document.id]);

  useEffect(() => {
    const element = viewerRef.current;
    if (!element) return;
    const observer = new ResizeObserver(() => setWidth(Math.max(320, Math.min(1100, element.clientWidth - 48))));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const highlightsByPage = useMemo(() => {
    const grouped = new Map<number, Array<{ highlight: DocumentHighlightDto; rect: PdfRect }>>();
    for (const highlight of highlights) {
      for (const rect of getPdfRects(highlight.position_data)) {
        const pageHighlights = grouped.get(rect.pageNumber) ?? [];
        pageHighlights.push({ highlight, rect });
        grouped.set(rect.pageNumber, pageHighlights);
      }
    }
    return grouped;
  }, [highlights]);

  useEffect(() => {
    if (!jumpToHighlightId) return;
    const target = window.document.querySelector<HTMLElement>(`[data-highlight-id="${CSS.escape(jumpToHighlightId)}"]`);
    target?.scrollIntoView({ behavior: "smooth", block: "center" });
    target?.classList.add("ring-2", "ring-violet-400");
    const timeout = window.setTimeout(() => target?.classList.remove("ring-2", "ring-violet-400"), 2000);
    return () => window.clearTimeout(timeout);
  }, [jumpToHighlightId, highlights]);

  const captureSelection = useCallback((): void => {
    const selected = window.getSelection();
    if (!selected || selected.isCollapsed || !selected.toString().trim() || !viewerRef.current) return;
    const range = selected.getRangeAt(0);
    const rects: PdfRect[] = [];
    for (const rect of Array.from(range.getClientRects())) {
      const page = window.document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2)
        ?.closest<HTMLElement>(".react-pdf__Page");
      const pageNumber = Number(page?.dataset.pageNumber);
      if (!page || !Number.isInteger(pageNumber) || pageNumber < 1) continue;
      const pageRect = page.getBoundingClientRect();
      rects.push({
        pageNumber,
        x: (rect.left - pageRect.left) / pageRect.width,
        y: (rect.top - pageRect.top) / pageRect.height,
        width: rect.width / pageRect.width,
        height: rect.height / pageRect.height,
      });
    }
    if (!rects.length) return;
    const bounds = range.getBoundingClientRect();
    setSelection({
      content: selected.toString().trim(),
      pageNumber: rects[0]?.pageNumber ?? null,
      positionData: { rects },
    });
    setPopoverPosition({ x: bounds.left, y: bounds.bottom });
  }, []);

  const saveHighlight = async (
    pending: PendingDocumentHighlight,
    color: HighlightColor,
    note: string,
  ): Promise<string> => {
    const id = await dbService.createHighlight(
      document.id,
      pending.content,
      pending.pageNumber,
      pending.positionData,
      note || null,
      color,
    );
    setHighlights(await dbService.getHighlights(document.id));
    return id;
  };

  return (
    <div ref={viewerRef} className="relative h-full overflow-auto bg-[#111318] px-4 py-6" onMouseUp={captureSelection}>
      {error && <p role="alert" className="mx-auto max-w-3xl rounded-lg bg-rose-950/60 p-3 text-sm text-rose-200">{error}</p>}
      {!error && !fileBytes && <p className="text-center text-sm text-zinc-400">Loading PDF...</p>}
      {fileBytes && (
        <Document
          file={{ data: fileBytes }}
          onLoadSuccess={({ numPages }) => setPageCount(numPages)}
          onLoadError={(cause) => {
            console.error("Could not render PDF:", cause);
            setError(cause.message);
          }}
          loading={<p className="text-center text-sm text-zinc-400">Rendering PDF...</p>}
          className="flex flex-col items-center gap-5"
        >
          {Array.from({ length: pageCount }, (_, index) => {
            const pageNumber = index + 1;
            return (
              <div key={pageNumber} className="relative inline-block max-w-full shadow-xl">
                <Page pageNumber={pageNumber} width={width} renderAnnotationLayer renderTextLayer />
                <div className="pointer-events-none absolute inset-0">
                  {(highlightsByPage.get(pageNumber) ?? []).map(({ highlight, rect }, rectIndex) => (
                    <div
                      key={`${highlight.id}-${rectIndex}`}
                      data-highlight-id={highlight.id}
                      className="absolute rounded-sm transition-shadow"
                      style={{
                        left: `${rect.x * 100}%`,
                        top: `${rect.y * 100}%`,
                        width: `${rect.width * 100}%`,
                        height: `${rect.height * 100}%`,
                        backgroundColor: highlightColors[highlight.color] ?? highlightColors.yellow,
                      }}
                    />
                  ))}
                </div>
              </div>
            );
          })}
        </Document>
      )}
      <HighlightPopover
        selection={selection}
        position={popoverPosition}
        onSaveHighlight={saveHighlight}
        onSendToLibrary={(id, content) => {
          if (!boardId) return Promise.reject(new Error("Select a board before sending a highlight to the Library."));
          return onSendToLibrary(id, content, boardId);
        }}
        onClose={() => {
          setSelection(null);
          setPopoverPosition(null);
        }}
      />
    </div>
  );
};
