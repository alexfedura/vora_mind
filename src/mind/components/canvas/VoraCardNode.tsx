import React, { memo, useRef, useLayoutEffect, useEffect, useState } from "react";
import { Handle, Position, NodeProps } from "@xyflow/react";
import {
  Clock,
  Maximize2,
  ArrowUpRight,
  Bookmark,
  Tag,
  Info,
  Trash2,
} from "lucide-react";
import { useAppStore } from "@/shared/store/useAppStore";
import { createTextChangeSet, dbService } from "@/mind/services/dbService";
import { MarkdownEditor } from "@/features/markdown-editor/MarkdownEditor";
import { renderMarkdown } from "@/features/markdown-editor/markdown/renderer";
import { useDocumentStore } from "@/features/file-store/useDocumentStore";
import { TextLinkPicker } from "./TextLinkPicker";
import type { TextLink } from "@/mind/types";

const resolveTextDirection = (text: string, defaultMajorRtl: boolean): { isRtl: boolean; cleanText: string } => {
  const trimmed = text.trim();
  const hasInvert = trimmed.startsWith("!");
  const cleanText = hasInvert ? trimmed.slice(1).trim() : text;

  const rtlRegex = /[\u0600-\u06FF\u0750-\u077F\u08A0-\u08FF\uFB50-\uFDFF\uFE70-\uFEFF]/;
  const hasRtlChar = rtlRegex.test(cleanText);

  let isRtl = hasRtlChar || defaultMajorRtl;
  if (hasInvert) isRtl = !isRtl;

  return { isRtl, cleanText };
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export const VoraCardNode = memo(({ id, selected, data }: NodeProps) => {
  const {
    cards,
    updateCardContent,
    updateCardVersion,
    activeBoardId,
    isRightDrawerOpen,
    toggleRightDrawer,
    textDirection,
  } = useAppStore();

  const isMajorRtl = textDirection === "rtl";
  const realCardId = (data?.cardId as string) || id;
  const currentDocId = useDocumentStore((state) => state.currentDocId);
  const isDocumentLoading = useDocumentStore((state) => state.isLoading);
  const documentError = useDocumentStore((state) => state.lastError);
  const isMarkdownEditorOpen = currentDocId === realCardId;
  const card = cards[realCardId] || {
    id: realCardId,
    title: (data?.title as string) || (data?.label as string) || "New card",
    content: (data?.content as string) || "",
    version: (data?.version as string) || "v0.1.0",
    boardId: activeBoardId,
  };

  const titleRef = useRef<HTMLTextAreaElement>(null);
  const [titleLinkRange, setTitleLinkRange] = useState<{ start: number; end: number } | null>(null);
  const [textLinks, setTextLinks] = useState<TextLink[]>([]);
  const [linkedTextById, setLinkedTextById] = useState<Record<string, string>>({});
  const [linkedSourceTitleById, setLinkedSourceTitleById] = useState<Record<string, string>>({});

  useEffect(() => {
    let cancelled = false;
    const loadTextLinks = (): void => {
      void dbService.getTextLinksForCard(activeBoardId, realCardId).then((loadedLinks) => {
        if (cancelled) return;
        setTextLinks(loadedLinks);
        const incomingLinks = loadedLinks.filter((link) =>
          link.target_board_id === activeBoardId && link.target_card_id === realCardId,
        );
        setLinkedTextById({});
        setLinkedSourceTitleById({});
        const sourceBoardIds = [...new Set(incomingLinks.map((link) => link.source_board_id))];
        void Promise.all(sourceBoardIds.map(async (boardId) => ({
          boardId,
          cards: await dbService.getAllCards(boardId),
        }))).then((boardCards) => {
          if (cancelled) return;
          const cardsByEndpoint = new Map<string, { title: string; content: string }>();
          for (const { boardId, cards } of boardCards) {
            for (const sourceCard of cards) {
              cardsByEndpoint.set(`${boardId}:${sourceCard.id}`, sourceCard);
            }
          }
          const snippets: Record<string, string> = {};
          const sourceTitles: Record<string, string> = {};
          for (const link of incomingLinks) {
            const sourceCard = cardsByEndpoint.get(`${link.source_board_id}:${link.source_card_id}`);
            const text = sourceCard?.[link.source_field];
            if (sourceCard) sourceTitles[link.id] = sourceCard.title || "Untitled card";
            if (text !== undefined) {
              const excerpt = text.slice(link.source_start, link.source_end);
              snippets[link.id] = excerpt.length > 60 ? `${excerpt.slice(0, 60)}…` : excerpt;
            }
          }
          setLinkedTextById(snippets);
          setLinkedSourceTitleById(sourceTitles);
        }).catch((error: unknown) => {
          console.error("Failed to load linked source text:", error);
        });
      }).catch((error: unknown) => {
        console.error("Failed to load card text links:", error);
      });
    };
    const handleTextLinksUpdated = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const link = isRecord(event.detail.link) ? event.detail.link : null;
      const affectsCard = event.detail.cardId === realCardId || (
        link !== null && (
          (link.source_board_id === activeBoardId && link.source_card_id === realCardId) ||
          (link.target_board_id === activeBoardId && link.target_card_id === realCardId)
        )
      );
      if (affectsCard) loadTextLinks();
    };
    loadTextLinks();
    window.addEventListener("vora:text-links-updated", handleTextLinksUpdated);
    return () => {
      cancelled = true;
      window.removeEventListener("vora:text-links-updated", handleTextLinksUpdated);
    };
  }, [activeBoardId, realCardId]);

  useEffect(() => {
    const handleSelectLinkedTitle = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const { cardId, field, start, end } = event.detail;
      if (
        cardId !== realCardId ||
        field !== "title" ||
        typeof start !== "number" ||
        typeof end !== "number" ||
        !titleRef.current
      ) return;
      titleRef.current.focus();
      titleRef.current.setSelectionRange(start, end);
    };
    window.addEventListener("vora:select-text-range", handleSelectLinkedTitle);
    return () => window.removeEventListener("vora:select-text-range", handleSelectLinkedTitle);
  }, [realCardId]);

  useEffect(() => {
    const handleCreateTitleLink = (event: Event): void => {
      if (!(event instanceof CustomEvent) || !isRecord(event.detail)) return;
      const detail = event.detail;
      if (
        detail.cardId !== realCardId ||
        detail.field !== "title" ||
        typeof detail.start !== "number" ||
        typeof detail.end !== "number" ||
        !Number.isInteger(detail.start) ||
        !Number.isInteger(detail.end) ||
        detail.start < 0 ||
        detail.end <= detail.start ||
        detail.end > card.title.length
      ) return;
      setTitleLinkRange({ start: detail.start, end: detail.end });
    };
    window.addEventListener("vora:create-text-link", handleCreateTitleLink);
    return () => window.removeEventListener("vora:create-text-link", handleCreateTitleLink);
  }, [card.title.length, realCardId]);

  const titleDirection = resolveTextDirection(card.title || "", isMajorRtl);
  const contentDirection = resolveTextDirection(card.content || "", isMajorRtl);

  useLayoutEffect(() => {
    if (titleRef.current) {
      titleRef.current.style.height = "0px";
      titleRef.current.style.height = `${titleRef.current.scrollHeight}px`;
    }
  }, [card.title, card.content]);

  useEffect(() => {
    if (!selected || card.title !== "New card") return;
    titleRef.current?.focus();
    titleRef.current?.select();
  }, [card.title, selected]);

  const handleOpenInLibrary = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!isRightDrawerOpen) toggleRightDrawer();
    window.dispatchEvent(new CustomEvent("vora:select-card", { detail: { cardId: realCardId } }));
  };

  const handleOpenFullscreen = (e: React.MouseEvent) => {
    e.stopPropagation();
    window.dispatchEvent(new CustomEvent("vora:open-fullscreen", { detail: { cardId: realCardId } }));
  };

  const handleRemoveFromCanvas = (e: React.MouseEvent) => {
    e.stopPropagation();
    window.dispatchEvent(new CustomEvent("vora:remove-from-canvas", { detail: { nodeId: id } }));
  };

  const handleOpenMarkdownEditor = (): void => {
    void useDocumentStore.getState().loadDoc(realCardId).catch((error: unknown) => {
      console.error("Failed to open card in Markdown editor:", error);
    });
  };

  const handleContentFocus = (): void => {
    if (!isMarkdownEditorOpen) handleOpenMarkdownEditor();
  };

  const handleTitleLinkKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>): void => {
    if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== "k") return;
    const target = event.currentTarget;
    if (target.selectionEnd <= target.selectionStart) return;
    event.preventDefault();
    void useDocumentStore.getState().saveDoc().then(() => {
      setTitleLinkRange({ start: target.selectionStart, end: target.selectionEnd });
    }).catch((error: unknown) => {
      console.error("Could not save card before creating title link:", error);
    });
  };

  const handleTitleContextMenu = (event: React.MouseEvent<HTMLTextAreaElement>): void => {
    const { selectionStart: start, selectionEnd: end } = event.currentTarget;
    if (end <= start) return;
    event.preventDefault();
    event.stopPropagation();
    window.dispatchEvent(new CustomEvent("vora:open-text-context-menu", {
      detail: { cardId: realCardId, field: "title", start, end, x: event.clientX, y: event.clientY },
    }));
  };

  const navigateFromCardLink = (link: TextLink, incoming: boolean): void => {
    window.dispatchEvent(new CustomEvent("vora:navigate-text-link", {
      detail: incoming
        ? {
          boardId: link.source_board_id,
          cardId: link.source_card_id,
          field: link.source_field,
          start: link.source_start,
          end: link.source_end,
        }
        : {
          boardId: link.target_board_id,
          cardId: link.target_card_id,
          field: link.target_field,
          start: link.target_start,
          end: link.target_end,
        },
    }));
  };

  const handleTitleLinkClick = (event: React.MouseEvent<HTMLTextAreaElement>): void => {
    const position = event.currentTarget.selectionStart;
    const sourceLinks = textLinks.filter((link) =>
      link.source_board_id === activeBoardId &&
      link.source_card_id === realCardId &&
      link.source_field === "title" &&
      position >= link.source_start &&
      position < link.source_end,
    );
    const matchingLinks = sourceLinks.length > 0
      ? sourceLinks
      : textLinks.filter((link) =>
        link.source_board_id === activeBoardId &&
        link.source_card_id === realCardId &&
        link.source_field === "title" &&
        position > 0 &&
        position - 1 >= link.source_start &&
        position - 1 < link.source_end,
      );
    if (matchingLinks.length === 1) navigateFromCardLink(matchingLinks[0], false);
  };

  return (
    <div
      className={`group relative w-88 rounded-2xl bg-[#1e2025] border px-6 pt-4 pb-6 flex flex-col gap-4 will-change-transform ${
        selected
          ? "border-[#383e4d]"
          : "border-[#282c36] hover:border-[#383e4d]"
      }`}
    >
      <Handle type="source" position={Position.Left} id="source-left" className={`!w-3 !h-3 !bg-[#e8ad55] !border-2 !border-[#1e2025] !-left-1.5 transition-opacity ${selected ? "!opacity-100 !pointer-events-auto" : "!opacity-0 !pointer-events-none"}`} />
      <Handle type="source" position={Position.Right} id="source-right" className={`!w-3 !h-3 !bg-[#e8ad55] !border-2 !border-[#1e2025] !-right-1.5 transition-opacity ${selected ? "!opacity-100 !pointer-events-auto" : "!opacity-0 !pointer-events-none"}`} />
      <Handle type="source" position={Position.Top} id="source-top" className={`!w-3 !h-3 !bg-[#e8ad55] !border-2 !border-[#1e2025] !-top-1.5 transition-opacity ${selected ? "!opacity-100 !pointer-events-auto" : "!opacity-0 !pointer-events-none"}`} />
      <Handle type="source" position={Position.Bottom} id="source-bottom" className={`!w-3 !h-3 !bg-[#e8ad55] !border-2 !border-[#1e2025] !-bottom-1.5 transition-opacity ${selected ? "!opacity-100 !pointer-events-auto" : "!opacity-0 !pointer-events-none"}`} />
      <Handle type="target" position={Position.Left} id="target-left" className="!w-3 !h-3 !-left-1.5 !opacity-0" />
      <Handle type="target" position={Position.Right} id="target-right" className="!w-3 !h-3 !-right-1.5 !opacity-0" />
      <Handle type="target" position={Position.Top} id="target-top" className="!w-3 !h-3 !-top-1.5 !opacity-0" />
      <Handle type="target" position={Position.Bottom} id="target-bottom" className="!w-3 !h-3 !-bottom-1.5 !opacity-0" />

      {/* نوار ابزار بالای کارت */}
      <div className={`w-full min-h-[26px] flex items-center justify-between select-none transition-opacity ${selected ? "opacity-100" : "opacity-0 pointer-events-none"}`}>
        {/* سمت چپ: بج نسخه با تایپوگرافی مدرن و متقارن */}
        <div className="flex items-center gap-2 text-gray-400">
          <input
            type="text"
            value={card.version || "v0.1.0"}
            onChange={(e) => {
              if (isMarkdownEditorOpen) {
                const appState = useAppStore.getState();
                const currentCard = appState.cards[realCardId];
                if (currentCard) {
                  appState.setCards({
                    ...appState.cards,
                    [realCardId]: { ...currentCard, version: e.target.value },
                  });
                }
                useDocumentStore.getState().setVersion(e.target.value);
              } else {
                updateCardVersion(realCardId, e.target.value);
              }
            }}
            style={{ fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', ui-monospace, monospace" }}
            className="nodrag w-14 text-center text-[11px] font-medium tracking-tight text-gray-300 bg-[#16181f] hover:bg-[#222530] focus:bg-[#272b38] focus:text-white focus:outline-none rounded-lg px-2 py-0.5 border border-[#2c3140] transition-colors cursor-text shadow-sm"
            title="Card Version"
          />
          <button className="p-1 hover:text-white hover:bg-[#252834] rounded transition" title="Bookmark">
            <Bookmark size={14} />
          </button>
          <button className="p-1 hover:text-white hover:bg-[#252834] rounded transition" title="Tags">
            <Tag size={14} />
          </button>
          <button className="p-1 hover:text-white hover:bg-[#252834] rounded transition" title="Info">
            <Info size={14} />
          </button>
          <button
            onClick={handleRemoveFromCanvas}
            className="p-1 hover:bg-rose-500/20 text-gray-400 hover:text-rose-400 rounded transition"
            title="Remove from Canvas"
          >
            <Trash2 size={14} />
          </button>
        </div>

        {/* سمت راست: اکشن‌های ناوبری */}
        <div className="flex items-center gap-1 text-gray-400">
          <button className="p-1 hover:text-white hover:bg-[#252834] rounded transition" title="History">
            <Clock size={14} />
          </button>
          <button
            onClick={handleOpenFullscreen}
            className="p-1 hover:text-white hover:bg-[#252834] rounded transition"
            title="Expand to Fullscreen"
          >
            <Maximize2 size={14} />
          </button>
          <button
            onClick={handleOpenInLibrary}
            className="p-1 hover:text-purple-400 hover:bg-[#252834] rounded transition"
            title="Open in Library Editor"
          >
            <ArrowUpRight size={14} />
          </button>
        </div>
      </div>

      {/* عنوان کارت */}
      <textarea
        ref={titleRef}
        value={card.title}
        onFocus={(event) => {
          if (card.title === "New card") event.currentTarget.select();
        }}
        onKeyDown={handleTitleLinkKeyDown}
        onClick={handleTitleLinkClick}
        onContextMenu={handleTitleContextMenu}
        onChange={(e) => {
          if (isMarkdownEditorOpen) {
            const appState = useAppStore.getState();
            const currentCard = appState.cards[realCardId];
            if (currentCard) {
              appState.setCards({
                ...appState.cards,
                [realCardId]: { ...currentCard, title: e.target.value },
              });
            }
            const changeSet = createTextChangeSet("title", card.title, e.target.value);
            useDocumentStore.getState().setTitle(e.target.value, changeSet ?? undefined);
          } else {
            updateCardContent(realCardId, e.target.value, card.content);
          }
          if (titleRef.current) {
            titleRef.current.style.height = "0px";
            titleRef.current.style.height = `${titleRef.current.scrollHeight}px`;
          }
        }}
        placeholder={titleDirection.isRtl ? "عنوان کارت..." : "Card title..."}
        rows={1}
        className={`nodrag w-full bg-transparent text-lg font-bold text-gray-100 placeholder-gray-500 resize-none overflow-hidden focus:outline-none leading-snug break-words ${
          titleDirection.isRtl ? "text-right" : "text-left"
        }`}
        dir={titleDirection.isRtl ? "rtl" : "ltr"}
      />

      {/* توضیحات کارت */}
      {isMarkdownEditorOpen ? (
        isDocumentLoading ? (
          <div className="nodrag min-h-32 text-sm text-gray-400">Loading card...</div>
        ) : documentError ? (
          <div role="alert" className="nodrag text-sm text-rose-400">
            {documentError}
            <button
              type="button"
              onClick={handleOpenMarkdownEditor}
              className="ml-2 underline"
            >
              Retry
            </button>
          </div>
        ) : (
          <MarkdownEditor
            docId={realCardId}
            className="vm-card-markdown-editor nodrag nowheel min-h-12 overflow-hidden rounded-lg"
            placeholder={contentDirection.isRtl ? "توضیحات و یادداشت..." : "Notes & content..."}
            direction={contentDirection.isRtl ? "rtl" : "ltr"}
          />
        )
      ) : (
        <div
          data-card-content-preview
          className={`vm-markdown vm-card-markdown-preview nodrag nowheel min-h-8 w-full cursor-text ${
            card.content ? "" : "text-gray-500"
          } ${contentDirection.isRtl ? "text-right" : "text-left"}`}
          dir={contentDirection.isRtl ? "rtl" : "ltr"}
          role="textbox"
          aria-readonly="true"
          aria-label="Card notes"
          tabIndex={0}
          onFocus={handleContentFocus}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              handleContentFocus();
            }
          }}
          dangerouslySetInnerHTML={{
            __html: card.content ? renderMarkdown(card.content) : (contentDirection.isRtl ? "توضیحات و یادداشت..." : "Notes & content..."),
          }}
        />
      )}

      {textLinks.some((link) =>
        link.target_board_id === activeBoardId && link.target_card_id === realCardId,
      ) && (
        <section className="nodrag rounded-lg border border-violet-500/30 bg-violet-500/[0.06] p-2 text-xs text-gray-300">
          <h3 className="mb-1.5 px-1 font-medium text-violet-300">Backlinks · click to return to the source</h3>
          <div className="flex max-h-32 flex-col gap-1 overflow-y-auto">
            {textLinks.filter((link) =>
              link.target_board_id === activeBoardId && link.target_card_id === realCardId,
            ).map((link) => (
              <button
                key={link.id}
                type="button"
                onClick={() => navigateFromCardLink(link, true)}
                className="flex min-w-0 items-center gap-2 rounded-md px-2 py-1.5 text-left hover:bg-[#303440] hover:text-white"
                title={`Return to ${linkedSourceTitleById[link.id] ?? link.source_card_id}, ${link.source_field}, characters ${link.source_start + 1}-${link.source_end}`}
              >
                <span aria-hidden="true" className="shrink-0 text-violet-300">↶</span>
                <span className="min-w-0 truncate">
                  {linkedSourceTitleById[link.id] ?? link.source_card_id}
                  <span className="text-gray-500"> · {link.source_field}</span>
                  {linkedTextById[link.id] && <span className="text-gray-300"> · “{linkedTextById[link.id]}”</span>}
                </span>
              </button>
            ))}
          </div>
        </section>
      )}

      {textLinks.some((link) =>
        link.source_board_id === activeBoardId && link.source_card_id === realCardId,
      ) && (
        <details className="nodrag rounded-lg border border-[#303440] bg-[#191b20] p-2 text-xs text-gray-300">
          <summary className="cursor-pointer select-none">
            Outgoing links ({textLinks.filter((link) =>
              link.source_board_id === activeBoardId && link.source_card_id === realCardId,
            ).length})
          </summary>
          <div className="mt-2 flex max-h-32 flex-col gap-1 overflow-y-auto">
            {textLinks.filter((link) =>
              link.source_board_id === activeBoardId && link.source_card_id === realCardId,
            ).map((link) => (
                <button
                  key={link.id}
                  type="button"
                  onClick={() => navigateFromCardLink(link, false)}
                  className="truncate rounded px-2 py-1 text-left hover:bg-[#303440] hover:text-white"
                  title={`Open ${link.target_card_id}, ${link.target_field}`}
                >
                  → {link.target_card_id} · {link.target_field}
                </button>
              ))}
          </div>
        </details>
      )}

      {titleLinkRange && (
        <TextLinkPicker
          sourceBoardId={activeBoardId}
          sourceCardId={realCardId}
          sourceField="title"
          sourceStart={titleLinkRange.start}
          sourceEnd={titleLinkRange.end}
          sourceText={card.title}
          onClose={() => setTitleLinkRange(null)}
          onCreated={(link) => {
            setTextLinks((current) => [...current, link]);
            window.dispatchEvent(new CustomEvent("vora:text-links-updated", {
              detail: { cardId: realCardId, link },
            }));
          }}
        />
      )}
    </div>
  );
});

VoraCardNode.displayName = "VoraCardNode";
export default VoraCardNode;
