import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import DOMPurify from "dompurify";
import { dbService, type DocumentDto, type DocumentHighlightDto } from "@/mind/services/dbService";
import { HighlightPopover, type HighlightColor, type PendingDocumentHighlight } from "./HighlightPopover";
import { getTextOffsets, getTextRects, type TextOffsets, type TextRect } from "./textSelection";

interface TextDocumentViewerProps {
  document: DocumentDto;
  boardId: string | null;
  jumpToHighlightId: string | null;
  onSendToLibrary: (highlightId: string, content: string, boardId: string) => Promise<void>;
}

interface PositionedHighlight {
  highlight: DocumentHighlightDto;
  rect: TextRect;
}

function parseOffsets(value: unknown): TextOffsets | null {
  if (typeof value !== "object" || value === null || !("startOffset" in value) || !("endOffset" in value)) return null;
  const startOffset = value.startOffset;
  const endOffset = value.endOffset;
  if (typeof startOffset !== "number" || typeof endOffset !== "number" || startOffset < 0 || endOffset <= startOffset) return null;
  return { startOffset, endOffset };
}

const highlightColors: Record<string, string> = {
  yellow: "rgba(250, 204, 21, .4)",
  green: "rgba(52, 211, 153, .36)",
  blue: "rgba(56, 189, 248, .36)",
  pink: "rgba(244, 114, 182, .36)",
};

export const TextDocumentViewer: React.FC<TextDocumentViewerProps> = ({
  document,
  boardId,
  jumpToHighlightId,
  onSendToLibrary,
}) => {
  const [html, setHtml] = useState("");
  const [highlights, setHighlights] = useState<DocumentHighlightDto[]>([]);
  const [selection, setSelection] = useState<PendingDocumentHighlight | null>(null);
  const [popoverPosition, setPopoverPosition] = useState<{ x: number; y: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [layoutRevision, setLayoutRevision] = useState(0);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    setError(null);
    setHtml("");
    void Promise.all([
      dbService.getDocumentContent(document.id),
      dbService.getHighlights(document.id),
    ]).then(async ([bytes, loadedHighlights]) => {
      let rendered: string;
      if (document.file_type === "docx") {
        const mammoth = await import("mammoth");
        const arrayBuffer = new ArrayBuffer(bytes.byteLength);
        new Uint8Array(arrayBuffer).set(bytes);
        const result = await mammoth.convertToHtml({ arrayBuffer });
        rendered = DOMPurify.sanitize(result.value);
      } else {
        const markdown = new TextDecoder().decode(bytes);
        const MarkdownIt = (await import("markdown-it")).default;
        rendered = DOMPurify.sanitize(new MarkdownIt({ html: false, linkify: true, breaks: true }).render(markdown));
      }
      if (cancelled) return;
      setHtml(rendered);
      setHighlights(loadedHighlights);
    }).catch((cause: unknown) => {
      if (!cancelled) {
        console.error("Could not load document content:", cause);
        setError(cause instanceof Error ? cause.message : String(cause));
      }
    });
    return () => { cancelled = true; };
  }, [document.id, document.file_type]);

  useEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    const observer = new ResizeObserver(() => setLayoutRevision((value) => value + 1));
    observer.observe(root);
    return () => observer.disconnect();
  }, []);

  const positionedHighlights = useMemo(() => {
    const root = rootRef.current;
    if (!root) return [];
    const result: PositionedHighlight[] = [];
    for (const highlight of highlights) {
      const offsets = parseOffsets(highlight.position_data);
      if (!offsets) continue;
      for (const rect of getTextRects(root, offsets.startOffset, offsets.endOffset)) {
        result.push({ highlight, rect });
      }
    }
    return result;
  }, [highlights, html, layoutRevision]);

  useEffect(() => {
    if (!jumpToHighlightId) return;
    const target = window.document.querySelector<HTMLElement>(`[data-highlight-id="${CSS.escape(jumpToHighlightId)}"]`);
    target?.scrollIntoView({ behavior: "smooth", block: "center" });
    target?.classList.add("ring-2", "ring-violet-400");
    const timeout = window.setTimeout(() => target?.classList.remove("ring-2", "ring-violet-400"), 2000);
    return () => window.clearTimeout(timeout);
  }, [jumpToHighlightId, positionedHighlights]);

  const captureSelection = useCallback((): void => {
    const root = rootRef.current;
    const selected = window.getSelection();
    if (!root || !selected || selected.isCollapsed || !selected.toString().trim()) return;
    const range = selected.getRangeAt(0);
    const offsets = getTextOffsets(root, range);
    if (!offsets) return;
    const bounds = range.getBoundingClientRect();
    setSelection({
      content: selected.toString().trim(),
      pageNumber: null,
      positionData: offsets,
    });
    setPopoverPosition({ x: bounds.left, y: bounds.bottom });
  }, []);

  const saveHighlight = async (
    pending: PendingDocumentHighlight,
    color: HighlightColor,
    note: string,
  ): Promise<string> => {
    const id = await dbService.createHighlight(document.id, pending.content, null, pending.positionData, note || null, color);
    setHighlights(await dbService.getHighlights(document.id));
    return id;
  };

  return (
    <div className="relative h-full overflow-auto bg-[#111318] px-8 py-8" onMouseUp={captureSelection}>
      {error && <p role="alert" className="mx-auto max-w-3xl rounded-lg bg-rose-950/60 p-3 text-sm text-rose-200">{error}</p>}
      <div
        ref={rootRef}
        className="relative mx-auto min-h-full max-w-4xl bg-[#191c24] px-10 py-12 text-[15px] leading-7 text-zinc-200 shadow-xl [&_a]:text-violet-300 [&_blockquote]:border-l-2 [&_blockquote]:border-zinc-600 [&_blockquote]:pl-4 [&_h1]:mb-6 [&_h1]:text-3xl [&_h2]:mb-4 [&_h2]:mt-8 [&_h2]:text-2xl [&_li]:ml-6 [&_ol]:list-decimal [&_p]:my-4 [&_pre]:overflow-auto [&_pre]:rounded-lg [&_pre]:bg-[#101217] [&_pre]:p-4 [&_ul]:list-disc"
        onScroll={() => setLayoutRevision((value) => value + 1)}
        dangerouslySetInnerHTML={{ __html: html }}
      />
      {positionedHighlights.map(({ highlight, rect }, index) => (
        <div
          key={`${highlight.id}-${index}`}
          data-highlight-id={highlight.id}
          className="pointer-events-none absolute rounded-sm transition-shadow"
          style={{
            left: `${rect.left + 32}px`,
            top: `${rect.top + 32}px`,
            width: `${rect.width}px`,
            height: `${rect.height}px`,
            backgroundColor: highlightColors[highlight.color] ?? highlightColors.yellow,
          }}
        />
      ))}
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
