import { useEffect, useRef, useState } from "react";
import type { ReactElement } from "react";
import {
  EditorView,
  keymap,
  placeholder as editorPlaceholder,
  type ViewUpdate,
} from "@codemirror/view";
import { createEditorState } from "./cm/setup";
import { livePreviewExtension } from "./cm/live-preview";
import { getEditorTextLinks, replaceTextLinks, textLinkExtension } from "./cm/text-links";
import { useDocumentStore } from "@/features/file-store/useDocumentStore";
import { dbService } from "@/mind/services/dbService";
import type { TextChangeRange, TextLink } from "@/mind/types";
import { TextLinkPicker } from "@/mind/components/canvas/TextLinkPicker";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export interface MarkdownEditorProps {
  docId: string;
  className?: string;
  placeholder?: string;
  direction?: "auto" | "ltr" | "rtl";
}

export function MarkdownEditor({
  docId,
  className = "",
  placeholder = "Start writing...",
  direction = "auto",
}: MarkdownEditorProps): ReactElement {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const linksRef = useRef<TextLink[]>([]);
  const [pickerRange, setPickerRange] = useState<{ from: number; to: number } | null>(null);
  const [activeLinks, setActiveLinks] = useState<TextLink[]>([]);
  const [linkMenuPosition, setLinkMenuPosition] = useState<{ x: number; y: number } | null>(null);
  const sourceBoardId = useDocumentStore((state) => state.currentBoardId);
  const sourceText = useDocumentStore((state) => state.content);

  const updateLinks = (nextLinks: TextLink[], view = viewRef.current): void => {
    linksRef.current = nextLinks;
    if (view) {
      window.setTimeout(() => {
        if (viewRef.current === view) view.dispatch({ effects: replaceTextLinks.of(nextLinks) });
      }, 0);
    }
  };

  const openLinkPicker = (from: number, to: number): void => {
    if (to <= from) return;
    void useDocumentStore.getState().saveDoc().then(() => {
      setPickerRange({ from, to });
    }).catch((error: unknown) => {
      console.error("Could not save card before creating text link:", error);
    });
  };

  const navigateToLink = (link: TextLink): void => {
    setActiveLinks([]);
    setLinkMenuPosition(null);
    window.dispatchEvent(new CustomEvent("vora:navigate-text-link", {
      detail: {
        boardId: link.target_board_id,
        cardId: link.target_card_id,
        field: link.target_field,
        start: link.target_start,
        end: link.target_end,
      },
    }));
  };

  useEffect(() => {
    const handleOutsidePointerDown = (event: PointerEvent): void => {
      if (!(event.target instanceof Element)) return;
      if (
        hostRef.current?.contains(event.target) ||
        event.target.closest("[data-text-link-picker], [data-card-content-preview]")
      ) return;

      window.setTimeout(() => {
        if (document.querySelector("[data-text-link-picker]")) return;
        const documentState = useDocumentStore.getState();
        if (documentState.currentDocId !== docId) return;
        void documentState.closeDoc().catch((error: unknown) => {
          console.error("Failed to save card after leaving its inline editor:", error);
        });
      }, 0);
    };

    document.addEventListener("pointerdown", handleOutsidePointerDown, true);
    return () => document.removeEventListener("pointerdown", handleOutsidePointerDown, true);
  }, [docId]);

  useEffect(() => {
    const host = hostRef.current;
    const documentState = useDocumentStore.getState();
    if (
      !host ||
      documentState.currentDocId !== docId ||
      documentState.currentBoardId === null ||
      documentState.isLoading ||
      documentState.lastError
    ) {
      return;
    }

    linksRef.current = [];
    setActiveLinks([]);
    setLinkMenuPosition(null);
    setPickerRange(null);
    let cancelled = false;
    void dbService.getTextLinksForCard(documentState.currentBoardId, docId).then((loadedLinks) => {
      if (cancelled) return;
      linksRef.current = loadedLinks;
      viewRef.current?.dispatch({ effects: replaceTextLinks.of(loadedLinks) });
    }).catch((error: unknown) => {
      console.error("Failed to load card text links:", error);
    });

    const linkKeymap = keymap.of([{
      key: "Mod-k",
      run: (view) => {
        const selection = view.state.selection.main;
        if (selection.empty) return false;
        openLinkPicker(selection.from, selection.to);
        return true;
      },
    }]);

    const updateListener = EditorView.updateListener.of((update: ViewUpdate) => {
      if (!update.docChanged) return;
      const currentState = useDocumentStore.getState();
      if (currentState.currentDocId === docId) {
        const changes: TextChangeRange[] = [];
        update.changes.iterChangedRanges((from, to, fromAfter, toAfter) => {
          const insertedText = update.state.doc.sliceString(fromAfter, toAfter);
          changes.push({
            from,
            to,
            inserted_length: insertedText.length,
            inserted_text: insertedText,
          });
        });
        const remappedLinks = remapLinksForContentEdit(linksRef.current, changes, docId);
        if (remappedLinks.length !== linksRef.current.length || changes.length > 0) {
          linksRef.current = remappedLinks;
          window.setTimeout(() => {
            if (viewRef.current === update.view) {
              update.view.dispatch({ effects: replaceTextLinks.of(remappedLinks) });
            }
          }, 0);
        }
        currentState.setContent(update.state.doc.toString(), {
          field: "content",
          changes,
        });
      }
    });

    const interactionHandlers = EditorView.domEventHandlers({
      contextmenu: (event, view) => {
        const selection = view.state.selection.main;
        if (selection.empty) return false;
        event.preventDefault();
        event.stopPropagation();
        window.dispatchEvent(new CustomEvent("vora:open-text-context-menu", {
          detail: {
            cardId: docId,
            field: "content",
            start: selection.from,
            end: selection.to,
            x: event.clientX,
            y: event.clientY,
          },
        }));
        return true;
      },
      click: (event, view) => {
        const position = view.posAtCoords({ x: event.clientX, y: event.clientY });
        if (position === null) return false;
        const sourceLinks = getEditorTextLinks(view).filter((link) =>
          link.source_field === "content" &&
          position >= link.source_start &&
          position < link.source_end,
        );
        const matchingLinks = sourceLinks.length > 0
          ? sourceLinks
          : getEditorTextLinks(view).filter((link) =>
            link.source_field === "content" &&
            position > 0 &&
            position - 1 >= link.source_start &&
            position - 1 < link.source_end,
          );
        if (matchingLinks.length === 0) return false;
        event.preventDefault();
        if (matchingLinks.length === 1) {
          navigateToLink(matchingLinks[0]);
          return true;
        }
        setActiveLinks(matchingLinks);
        setLinkMenuPosition({ x: event.clientX, y: event.clientY });
        return true;
      },
    });

    const state = createEditorState(documentState.content, {
      livePreview: livePreviewExtension,
      extra: [
        textLinkExtension,
        linkKeymap,
        updateListener,
        interactionHandlers,
        editorPlaceholder(placeholder),
      ],
    });
    const view = new EditorView({ state, parent: host });
    viewRef.current = view;
    view.dispatch({ effects: replaceTextLinks.of(linksRef.current) });
    view.focus();
    const handleSelectLinkedRange = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const detail = event.detail;
      if (
        detail.cardId !== docId ||
        detail.field !== "content" ||
        typeof detail.start !== "number" ||
        typeof detail.end !== "number"
      ) return;
      const start = Math.max(0, Math.min(view.state.doc.length, detail.start));
      const end = Math.max(start, Math.min(view.state.doc.length, detail.end));
      view.dispatch({ selection: { anchor: start, head: end }, scrollIntoView: true });
      view.focus();
    };
    window.addEventListener("vora:select-text-range", handleSelectLinkedRange);
    const handleCreateContentLink = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const detail = event.detail;
      if (
        detail.cardId !== docId ||
        detail.field !== "content" ||
        typeof detail.start !== "number" ||
        typeof detail.end !== "number" ||
        !Number.isInteger(detail.start) ||
        !Number.isInteger(detail.end) ||
        detail.start < 0 ||
        detail.end <= detail.start ||
        detail.end > view.state.doc.length
      ) return;
      openLinkPicker(detail.start, detail.end);
    };
    window.addEventListener("vora:create-text-link", handleCreateContentLink);

    return () => {
      cancelled = true;
      window.removeEventListener("vora:select-text-range", handleSelectLinkedRange);
      window.removeEventListener("vora:create-text-link", handleCreateContentLink);
      viewRef.current = null;
      view.destroy();
    };
  }, [docId, placeholder]);

  return (
    <div className="relative">
      <div ref={hostRef} dir={direction} className={`vm-editor-wrapper ${className}`} />
      {pickerRange && sourceBoardId && (
        <TextLinkPicker
          sourceBoardId={sourceBoardId}
          sourceCardId={docId}
          sourceField="content"
          sourceStart={pickerRange.from}
          sourceEnd={pickerRange.to}
          sourceText={sourceText}
          onClose={() => {
            setPickerRange(null);
            setLinkMenuPosition(null);
          }}
          onCreated={(link) => {
            updateLinks([...linksRef.current, link]);
            window.dispatchEvent(new CustomEvent("vora:text-links-updated", {
              detail: { cardId: docId, link },
            }));
          }}
        />
      )}
      {activeLinks.length > 0 && linkMenuPosition && (
        <div
          className="fixed z-[90] min-w-48 rounded-lg border border-[#383e4d] bg-[#1e2025] p-2 text-sm shadow-xl"
          style={{ left: linkMenuPosition.x, top: linkMenuPosition.y }}
          dir="auto"
        >
          {activeLinks.map((link) => (
            <button
              key={link.id}
              type="button"
              onClick={() => navigateToLink(link)}
              className="block w-full rounded px-3 py-2 text-left text-gray-200 hover:bg-[#303440]"
            >
              Open {link.target_field} · {link.target_card_id}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function remapRange(
  start: number,
  end: number,
  changes: TextChangeRange[],
): { start: number; end: number } | null {
  let delta = 0;
  for (const change of changes) {
    if (change.from === change.to) {
      if (change.from > start && change.from < end) return null;
      if (change.from <= start) delta += change.inserted_length;
    } else if (change.from < end && change.to > start) {
      return null;
    } else if (change.to <= start) {
      delta += change.inserted_length - (change.to - change.from);
    }
  }
  return { start: start + delta, end: end + delta };
}

function remapLinksForContentEdit(
  links: TextLink[],
  changes: TextChangeRange[],
  cardId: string,
): TextLink[] {
  return links.flatMap((link) => {
    const updated = { ...link };
    if (link.source_card_id === cardId && link.source_field === "content") {
      const range = remapRange(link.source_start, link.source_end, changes);
      if (!range) return [];
      updated.source_start = range.start;
      updated.source_end = range.end;
    }
    if (link.target_card_id === cardId && link.target_field === "content") {
      const range = remapRange(link.target_start, link.target_end, changes);
      if (!range) return [];
      updated.target_start = range.start;
      updated.target_end = range.end;
    }
    return [updated];
  });
}
