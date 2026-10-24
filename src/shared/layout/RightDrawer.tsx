import React, { useState, useRef, useLayoutEffect, useEffect } from "react";
import {
  PanelRight,
  Plus,
  Search,
  ChevronDown,
  Tag,
  Filter,
  ArrowUpDown,
  FileText,
  ChevronLeft,
  Sparkles,
  Trash2,
  RotateCcw,
  Archive,
  Layers,
  History,
  Pencil,
} from "lucide-react";
import { useAppStore } from "@/shared/store/useAppStore";
import type {
  LibraryItemGroup,
  LibraryItemGroupSnapshot,
  LibraryItemGroupUsage,
  LibraryItemGroupVersion,
  VoraCard,
} from "@/mind/types";

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

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

interface LibraryMediaPreviewData {
  kind: "image" | "video";
  src: string;
}

const resolveLibraryMediaPreview = (content: string): LibraryMediaPreviewData | null => {
  const trimmedContent = content.trim();
  const markdownUrl = /^!\[[^\]]*]\((https?:\/\/[^)\s]+)\)$/.exec(trimmedContent)?.[1];
  const directUrl = /^https?:\/\/\S+$/.exec(trimmedContent)?.[0];
  const src = markdownUrl ?? directUrl;
  if (!src) return null;

  try {
    const url = new URL(src);
    if (url.protocol !== "http:" && url.protocol !== "https:") return null;
    if (/\.(?:png|jpe?g|gif|webp|avif)$/i.test(url.pathname)) return { kind: "image", src };
    if (/\.(?:mp4|webm|ogv)$/i.test(url.pathname)) return { kind: "video", src };
  } catch {
    return null;
  }
  return null;
};

const LibraryMediaPreview: React.FC<{ media: LibraryMediaPreviewData }> = ({ media }) => (
  media.kind === "image" ? (
    <img
      src={media.src}
      alt="Library item preview"
      loading="lazy"
      draggable={false}
      className="pointer-events-auto max-h-20 w-full rounded object-contain"
    />
  ) : (
    <video
      src={media.src}
      controls
      playsInline
      preload="metadata"
      className="pointer-events-auto max-h-20 w-full rounded bg-black object-contain"
      onPointerDown={(event) => event.stopPropagation()}
      onClick={(event) => event.stopPropagation()}
    />
  )
);

const GroupSchematicPreview: React.FC<{
  snapshot: LibraryItemGroupSnapshot;
  className?: string;
  label: string;
}> = ({ snapshot, className, label }) => {
  if (snapshot.nodes.length === 0) return null;
  const nodes = snapshot.nodes;
  const minX = Math.min(...nodes.map((node) => node.x));
  const minY = Math.min(...nodes.map((node) => node.y));
  const maxX = Math.max(...nodes.map((node) => node.x + (node.width ?? 180)));
  const maxY = Math.max(...nodes.map((node) => node.y + (node.height ?? 100)));
  const width = Math.max(1, maxX - minX);
  const height = Math.max(1, maxY - minY);
  const nodeById = new Map(nodes.map((node) => [node.id, node]));

  return (
    <svg
      viewBox={`${minX} ${minY} ${width} ${height}`}
      preserveAspectRatio="xMidYMid meet"
      role="img"
      aria-label={label}
      className={className}
    >
      {snapshot.edges.map((edge) => {
        const source = nodeById.get(edge.source);
        const target = nodeById.get(edge.target);
        if (!source || !target) return null;
        return (
          <line
            key={edge.id}
            x1={source.x + (source.width ?? 180) / 2}
            y1={source.y + (source.height ?? 100) / 2}
            x2={target.x + (target.width ?? 180) / 2}
            y2={target.y + (target.height ?? 100) / 2}
            stroke="#a78bfa"
            strokeWidth={Math.max(1.5, width / 160)}
          />
        );
      })}
      {nodes.map((node) => {
        const nodeWidth = node.width ?? 180;
        const nodeHeight = node.height ?? 100;
        const shape = node.data.shape;
        const color = typeof node.data.color === "string" && /^#[\da-f]{6}$/i.test(node.data.color)
          ? node.data.color
          : "#e8ad55";
        const fillColor = typeof node.data.fillColor === "string" && /^#[\da-f]{6}$/i.test(node.data.fillColor)
          ? node.data.fillColor
          : color;
        const content = node.node_type === "voraText"
          ? typeof node.data.text === "string" ? node.data.text : node.content
          : node.content;
        const cardId = typeof node.data.cardId === "string" ? node.data.cardId : node.id;
        const card = snapshot.cards.find((item) => item.id === cardId);
        const title = card?.title || node.title || (typeof node.data.label === "string" ? node.data.label : "");
        if (node.node_type === "voraShape" && (shape === "rectangle" || shape === "path")) {
          const points = Array.isArray(node.data.points)
            ? node.data.points.filter((point): point is { x: number; y: number } =>
                isRecord(point) &&
                typeof point.x === "number" && Number.isFinite(point.x) &&
                typeof point.y === "number" && Number.isFinite(point.y)
              )
            : [];
          return shape === "rectangle" ? (
            <rect
              key={node.id}
              x={node.x}
              y={node.y}
              width={nodeWidth}
              height={nodeHeight}
              fill={fillColor}
              fillOpacity={typeof node.data.fillOpacity === "number" ? node.data.fillOpacity : 0.12}
              stroke={color}
              strokeWidth={Math.max(1.5, width / 160)}
            />
          ) : (
            <g key={node.id}>
              {points.slice(0, -1).map((point, index) => {
                const next = points[index + 1];
                if (!next) return null;
                const segment = Array.isArray(node.data.segments) ? node.data.segments[index] : null;
                const segmentColor = isRecord(segment) &&
                  typeof segment.color === "string" &&
                  /^#[\da-f]{6}$/i.test(segment.color)
                  ? segment.color
                  : color;
                return (
                  <path
                    key={`${node.id}:${index}`}
                    d={`M ${node.x + point.x * nodeWidth} ${node.y + point.y * nodeHeight} L ${node.x + next.x * nodeWidth} ${node.y + next.y * nodeHeight}`}
                    fill="none"
                    stroke={segmentColor}
                    strokeWidth={Math.max(1.5, width / 160)}
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                );
              })}
            </g>
          );
        }
        if (node.node_type === "voraText") {
          return (
            <text
              key={node.id}
              x={node.x + 4}
              y={node.y + 18}
              fill="#d6dbe2"
              fontSize={Math.max(10, width / 45)}
            >
              {content.slice(0, 36) || title || "Text"}
            </text>
          );
        }
        return (
          <g key={node.id}>
            <rect
              x={node.x}
              y={node.y}
              width={nodeWidth}
              height={nodeHeight}
              rx={Math.max(4, width / 180)}
              fill="#1c1b19"
              stroke="#5a5142"
              strokeWidth={Math.max(1.5, width / 160)}
            />
            <text
              x={node.x + nodeWidth / 2}
              y={node.y + Math.max(18, nodeHeight * 0.35)}
              textAnchor="middle"
              fill="#e7e5e4"
              fontSize={Math.max(10, width / 45)}
            >
              {title.slice(0, 28) || "Card"}
            </text>
            <text
              x={node.x + nodeWidth / 2}
              y={node.y + Math.max(34, nodeHeight * 0.58)}
              textAnchor="middle"
              fill="#a1a1aa"
              fontSize={Math.max(8, width / 60)}
            >
              {content.slice(0, 42)}
            </text>
          </g>
        );
      })}
    </svg>
  );
};

const GroupDragContentPreview: React.FC<{ snapshot: LibraryItemGroupSnapshot }> = ({ snapshot }) => {
  const cardsById = new Map(snapshot.cards.map((card) => [card.id, card]));
  return (
    <div className="mt-2 flex flex-col gap-1.5 border-t border-amber-400/20 pt-2">
      {snapshot.nodes.map((node) => {
        const cardId = typeof node.data.cardId === "string" ? node.data.cardId : node.id;
        const card = cardsById.get(cardId);
        const title = card?.title || node.title || (
          typeof node.data.label === "string" ? node.data.label : node.node_type
        );
        const content = node.node_type === "voraText" && typeof node.data.text === "string"
          ? node.data.text
          : card?.content ?? node.content;
        const segments = Array.isArray(node.data.segments)
          ? node.data.segments.filter(isRecord).map((segment) => {
              const name = typeof segment.name === "string" ? segment.name : "";
              const details = typeof segment.details === "string" ? segment.details : "";
              return [name, details].filter(Boolean).join(": ");
            }).filter(Boolean)
          : [];

        return (
          <article key={node.id} className="rounded-md bg-black/20 px-2 py-1.5">
            <h3 className="text-xs font-semibold text-zinc-100">{title || "Untitled item"}</h3>
            {content && (
              <p className="mt-0.5 whitespace-pre-wrap break-words text-[11px] leading-relaxed text-zinc-300">
                {content}
              </p>
            )}
            {segments.map((segment, index) => (
              <p key={`${node.id}-segment-${index}`} className="mt-0.5 whitespace-pre-wrap break-words text-[11px] leading-relaxed text-zinc-400">
                {segment}
              </p>
            ))}
          </article>
        );
      })}
    </div>
  );
};

type ViewFilter = "all" | "cards" | "item-groups" | "trash";
type SortBy = "alphabetical" | "created" | "viewed";
type SortOrder = "oldest" | "newest";

type LibraryDragPreview =
  | { kind: "card"; card: VoraCard }
  | { kind: "group"; group: LibraryItemGroup };

interface RightDrawerProps {
  selectedCardId: string | null;
  setSelectedCardId: (id: string | null) => void;
  onAddNewCard: () => void;
  onNodeDataSync: (id: string, title: string, content: string) => void;
  itemGroups: LibraryItemGroup[];
  itemGroupError: string | null;
  onMoveItemGroupToTrash: (groupId: string) => Promise<void>;
  onRestoreItemGroup: (groupId: string) => Promise<void>;
  onInsertItemGroup: (group: LibraryItemGroup, position?: { x: number; y: number }) => void;
  onGetItemGroupVersions: (groupId: string) => Promise<LibraryItemGroupVersion[]>;
  onGetItemGroupUsages: (groupId: string) => Promise<LibraryItemGroupUsage[]>;
  onOpenLibraryBoard: (
    group: LibraryItemGroup,
    snapshot: LibraryItemGroupSnapshot,
    baseVersion: string,
  ) => Promise<void>;
}

export const RightDrawer: React.FC<RightDrawerProps> = ({
  selectedCardId,
  setSelectedCardId,
  onAddNewCard,
  onNodeDataSync,
  itemGroups,
  itemGroupError,
  onMoveItemGroupToTrash,
  onRestoreItemGroup,
  onInsertItemGroup,
  onGetItemGroupVersions,
  onGetItemGroupUsages,
  onOpenLibraryBoard,
}) => {
  const {
    isRightDrawerOpen,
    toggleRightDrawer,
    cards,
    updateCardContent,
    updateCardVersion,
    moveToBin,
    restoreFromBin,
    textDirection,
  } = useAppStore();

  const isMajorRtl = textDirection === "rtl";

  const [currentFilter, setCurrentFilter] = useState<ViewFilter>("all");
  const [isFilterMenuOpen, setIsFilterMenuOpen] = useState(false);
  const [isSortMenuOpen, setIsSortMenuOpen] = useState(false);
  const [sortBy, setSortBy] = useState<SortBy>("alphabetical");
  const [sortOrder, setSortOrder] = useState<SortOrder>("oldest");
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedGroupId, setSelectedGroupId] = useState<string | null>(null);
  const [groupError, setGroupError] = useState<string | null>(null);
  const [groupVersions, setGroupVersions] = useState<LibraryItemGroupVersion[]>([]);
  const [groupUsages, setGroupUsages] = useState<LibraryItemGroupUsage[]>([]);
  const [selectedGroupVersionId, setSelectedGroupVersionId] = useState<string | null>(null);

  const [draggingLibraryItem, setDraggingLibraryItem] = useState<LibraryDragPreview | null>(null);
  const [ghostPos, setGhostPos] = useState({ x: 0, y: 0 });
  const isDraggingRef = useRef(false);
  const dragStartPos = useRef({ x: 0, y: 0 });
  const currentDraggingItemRef = useRef<LibraryDragPreview | null>(null);
  const suppressLibraryClickRef = useRef<{ kind: LibraryDragPreview["kind"]; id: string } | null>(null);
  const suppressCanvasClickRef = useRef(false);

  const sidebarTitleRef = useRef<HTMLTextAreaElement>(null);
  const sidebarContentRef = useRef<HTMLTextAreaElement>(null);

  const activeCard = selectedCardId ? cards[selectedCardId] : null;
  const activeGroup = selectedGroupId
    ? itemGroups.find((group) => group.id === selectedGroupId) ?? null
    : null;
  const selectedGroupVersion = groupVersions.find((version) => version.id === selectedGroupVersionId);
  const displayedGroupSnapshot = selectedGroupVersion?.snapshot ?? activeGroup?.snapshot;

  useEffect(() => {
    let cancelled = false;
    if (!activeGroup) {
      setGroupVersions([]);
      setSelectedGroupVersionId(null);
      return () => { cancelled = true; };
    }
    const loadVersions = async (): Promise<void> => {
      try {
        const versions = await onGetItemGroupVersions(activeGroup.id);
        if (cancelled) return;
        setGroupVersions(versions);
        const currentVersion = versions.find((version) => version.version === activeGroup.version);
        setSelectedGroupVersionId(currentVersion?.id ?? versions[0]?.id ?? null);
      } catch (cause) {
        console.error("Could not load item group version history:", cause);
        if (!cancelled) {
          setGroupError(cause instanceof Error ? cause.message : String(cause));
        }
      }
    };
    void loadVersions();
    return () => { cancelled = true; };
  }, [activeGroup?.id, activeGroup?.version]);

  useEffect(() => {
    let cancelled = false;
    if (!activeGroup) {
      setGroupUsages([]);
      return () => { cancelled = true; };
    }
    const loadUsages = async (): Promise<void> => {
      try {
        const usages = await onGetItemGroupUsages(activeGroup.id);
        if (!cancelled) setGroupUsages(usages);
      } catch (cause) {
        console.error("Could not load Library item usages:", cause);
        if (!cancelled) {
          setGroupError(cause instanceof Error ? cause.message : String(cause));
        }
      }
    };
    void loadUsages();
    return () => { cancelled = true; };
  }, [activeGroup?.id, onGetItemGroupUsages]);

  useLayoutEffect(() => {
    if (sidebarTitleRef.current) {
      sidebarTitleRef.current.style.height = "0px";
      sidebarTitleRef.current.style.height = `${sidebarTitleRef.current.scrollHeight}px`;
    }
  }, [activeCard?.title, selectedCardId]);

  useEffect(() => {
    const handlePointerMove = (e: PointerEvent) => {
      if (!isDraggingRef.current && currentDraggingItemRef.current) {
        const dx = Math.abs(e.clientX - dragStartPos.current.x);
        const dy = Math.abs(e.clientY - dragStartPos.current.y);
        if (dx > 4 || dy > 4) {
          isDraggingRef.current = true;
          setDraggingLibraryItem(currentDraggingItemRef.current);
          suppressLibraryClickRef.current = currentDraggingItemRef.current.kind === "card"
            ? { kind: "card", id: currentDraggingItemRef.current.card.id }
            : { kind: "group", id: currentDraggingItemRef.current.group.id };
        }
      }

      if (isDraggingRef.current) {
        setGhostPos({ x: e.clientX, y: e.clientY });
      }
    };

    const handlePointerUp = (e: PointerEvent) => {
      const draggedItem = currentDraggingItemRef.current;
      if (isDraggingRef.current && draggedItem) {
        const dropX = e.clientX;
        const dropY = e.clientY;
        const mainCanvas = document.querySelector("main");

        if (mainCanvas) {
          const rect = mainCanvas.getBoundingClientRect();
          const isInsideCanvas =
            dropX >= rect.left &&
            dropX <= rect.right &&
            dropY >= rect.top &&
            dropY <= rect.bottom;

          if (isInsideCanvas) {
            window.dispatchEvent(
              new CustomEvent(draggedItem.kind === "card"
                ? "vora:drop-card-to-canvas"
                : "vora:drop-item-group-to-canvas", {
                detail: {
                  id: draggedItem.kind === "card" ? draggedItem.card.id : draggedItem.group.id,
                  clientX: dropX,
                  clientY: dropY,
                },
              })
            );
            suppressCanvasClickRef.current = true;
            window.setTimeout(() => {
              suppressCanvasClickRef.current = false;
            }, 250);
          }
        }
      }

      isDraggingRef.current = false;
      currentDraggingItemRef.current = null;
      setDraggingLibraryItem(null);
      if (suppressLibraryClickRef.current) {
        const suppressedClick = suppressLibraryClickRef.current;
        window.setTimeout(() => {
          if (
            suppressLibraryClickRef.current?.kind === suppressedClick.kind &&
            suppressLibraryClickRef.current.id === suppressedClick.id
          ) {
            suppressLibraryClickRef.current = null;
          }
        }, 250);
      }
    };

    const handlePointerCancel = (): void => {
      isDraggingRef.current = false;
      currentDraggingItemRef.current = null;
      setDraggingLibraryItem(null);
    };

    const handleCanvasClickAfterDrop = (event: MouseEvent): void => {
      if (!suppressCanvasClickRef.current || !(event.target instanceof Node)) return;
      const mainCanvas = document.querySelector("main");
      if (!mainCanvas?.contains(event.target)) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      suppressCanvasClickRef.current = false;
    };

    window.addEventListener("pointermove", handlePointerMove);
    window.addEventListener("pointerup", handlePointerUp);
    window.addEventListener("pointercancel", handlePointerCancel);
    document.addEventListener("click", handleCanvasClickAfterDrop, true);

    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("pointercancel", handlePointerCancel);
      document.removeEventListener("click", handleCanvasClickAfterDrop, true);
    };
  }, []);

  const handleLibraryItemPointerDown = (e: React.PointerEvent, item: LibraryDragPreview) => {
    if (e.button !== 0) return;
    const targetButton = e.target instanceof Element ? e.target.closest("button") : null;
    if (targetButton && targetButton.getAttribute("aria-label")?.startsWith("Open item group") !== true) return;
    dragStartPos.current = { x: e.clientX, y: e.clientY };
    currentDraggingItemRef.current = item;
  };

  const handleLibraryItemClick = (
    event: React.MouseEvent,
    kind: LibraryDragPreview["kind"],
    id: string,
  ): boolean => {
    const suppressedClick = suppressLibraryClickRef.current;
    if (!suppressedClick || suppressedClick.kind !== kind || suppressedClick.id !== id) return false;
    suppressLibraryClickRef.current = null;
    event.preventDefault();
    event.stopPropagation();
    return true;
  };

  const handleMoveToTrash = (id: string, e?: React.MouseEvent) => {
    if (e) e.stopPropagation();
    moveToBin(id);
    window.dispatchEvent(new CustomEvent("vora:remove-from-canvas", { detail: { nodeId: id } }));
    if (selectedCardId === id) setSelectedCardId(null);
  };

  const handleRestore = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    restoreFromBin(id);
  };

  const filteredCards: VoraCard[] = Object.values(cards)
    .filter((c) => {
      if (c.librarySourceCardId) return false;
      const matchesSearch = (c.title + " " + c.content).toLowerCase().includes(searchQuery.toLowerCase());
      if (currentFilter === "all" || currentFilter === "cards") {
        return matchesSearch && !c.deleted_at;
      }
      if (currentFilter === "trash") {
        return matchesSearch && Boolean(c.deleted_at);
      }
      return false;
    })
    .sort((a, b) => {
      let result = 0;
      if (sortBy === "alphabetical") {
        const titleA = (a.title || "").trim();
        const titleB = (b.title || "").trim();
        result = titleA.localeCompare(titleB, "fa", { sensitivity: "base" });
        return sortOrder === "oldest" ? result : -result;
      }
      if (sortBy === "created" || sortBy === "viewed") {
        const timeA = a.created_at || 0;
        const timeB = b.created_at || 0;
        result = timeA - timeB;
        return sortOrder === "oldest" ? result : -result;
      }
      return 0;
    });

  const filteredGroups = itemGroups
    .filter((group) => {
      const matchesSearch = `${group.name} ${group.version}`.toLowerCase().includes(searchQuery.toLowerCase());
      if (currentFilter === "all" || currentFilter === "item-groups") {
        return matchesSearch && group.deleted_at === null;
      }
      if (currentFilter === "trash") return matchesSearch && group.deleted_at !== null;
      return false;
    })
    .sort((a, b) => b.created_at - a.created_at);
  const isLibraryEmpty = filteredCards.length === 0 && filteredGroups.length === 0;

  const handleInsertGroup = (group: LibraryItemGroup): void => {
    try {
      onInsertItemGroup(group);
      setGroupError(null);
    } catch (cause) {
      console.error("Could not add item group to canvas:", cause);
      setGroupError(cause instanceof Error ? cause.message : String(cause));
    }
  };

  const handleGroupTrashAction = async (
    groupId: string,
    action: "trash" | "restore",
  ): Promise<void> => {
    setGroupError(null);
    try {
      if (action === "trash") await onMoveItemGroupToTrash(groupId);
      else await onRestoreItemGroup(groupId);
    } catch (cause) {
      console.error(`Could not ${action} item group:`, cause);
      setGroupError(cause instanceof Error ? cause.message : String(cause));
    }
  };

  if (!isRightDrawerOpen) return null;

  const activeTitleDir = resolveTextDirection(activeCard?.title || "", isMajorRtl);
  const draggingCardMedia = draggingLibraryItem?.kind === "card"
    ? resolveLibraryMediaPreview(draggingLibraryItem.card.content)
    : null;

  return (
    <>
      {draggingLibraryItem && (
        <div
          style={{
            position: "fixed",
            left: `${Math.max(12, Math.min(
              ghostPos.x + 16,
              window.innerWidth - (draggingLibraryItem.kind === "group" ? 440 : 360),
            ))}px`,
            top: `${Math.max(12, Math.min(ghostPos.y + 16, window.innerHeight - 320))}px`,
            zIndex: 99999,
            pointerEvents: "none",
          }}
          className={`max-h-[70vh] overflow-auto rounded-2xl border-2 p-3 shadow-2xl opacity-95 backdrop-blur ${
            draggingLibraryItem.kind === "group"
              ? "w-[min(420px,calc(100vw-1.5rem))] border-amber-500 bg-[#1e1c18] shadow-amber-500/30"
              : "w-[min(340px,calc(100vw-1.5rem))] border-purple-500 bg-[#1e222b] shadow-purple-500/30"
          }`}
        >
          {draggingLibraryItem.kind === "card" ? (
            <>
              <div className="mb-2 flex items-center gap-2 border-b border-purple-400/20 pb-2 text-sm font-bold text-white">
                <FileText size={17} className="shrink-0 text-purple-400" />
                <span className="break-words">{draggingLibraryItem.card.title || "Untitled"}</span>
                <span className="ml-auto shrink-0 font-mono text-[10px] text-purple-300">
                  {draggingLibraryItem.card.version || "v0.1.0"}
                </span>
              </div>
              {draggingCardMedia && (
                <div className="mb-2">
                  <LibraryMediaPreview media={draggingCardMedia} />
                </div>
              )}
              <div className="whitespace-pre-wrap break-words text-sm leading-relaxed text-gray-200">
                {draggingLibraryItem.card.content || "No content"}
              </div>
            </>
          ) : (
            <>
              <div className="mb-2 flex items-center gap-2 border-b border-amber-400/20 pb-2 text-sm font-semibold text-amber-100">
                <Layers size={16} className="shrink-0 text-amber-300" />
                <span className="break-words">{draggingLibraryItem.group.name}</span>
                <span className="ml-auto shrink-0 text-[10px] text-zinc-400">
                  {draggingLibraryItem.group.snapshot.nodes.length} items · {draggingLibraryItem.group.snapshot.edges.length} connections
                </span>
              </div>
              <GroupSchematicPreview
                snapshot={draggingLibraryItem.group.snapshot}
                label={`Dragged schematic preview of ${draggingLibraryItem.group.name}`}
                className="h-[240px] w-full rounded-lg bg-[#141311]"
              />
              <GroupDragContentPreview snapshot={draggingLibraryItem.group.snapshot} />
            </>
          )}
        </div>
      )}

      <aside className="w-[26rem] flex-shrink-0 bg-background flex flex-col z-30 shadow-2xl transition-all border-l border-[#2a2a2a] select-none">
        <div className="h-14 px-5 flex items-center justify-between border-b border-[#20242f] bg-[#111317]">
          <span className="text-sm font-bold text-gray-200 tracking-wide">
            {activeCard ? "Card View" : activeGroup ? activeGroup.name : currentFilter === "trash" ? "Trash" : "Library"}
          </span>
          <button
            onClick={toggleRightDrawer}
            className="p-2 rounded-lg bg-[#1a1d24] hover:bg-[#252934] text-gray-400 hover:text-purple-400 transition"
            title="Close Panel"
          >
            <PanelRight size={18} />
          </button>
        </div>

        {activeCard ? (
          <div className="flex flex-col flex-1 overflow-y-auto no-scrollbar">
            <div className="px-5 py-2.5 flex items-center justify-between bg-[#171920] border-b border-[#20242f]">
              <button
                onClick={() => setSelectedCardId(null)}
                className="flex items-center gap-1.5 text-sm font-medium text-gray-300 hover:text-white px-3 py-1.5 rounded-lg bg-[#202430] hover:bg-[#282d3c] transition"
              >
                <ChevronLeft size={16} />
                <span>Library</span>
              </button>

              <div className="flex items-center gap-2">
                <input
                  type="text"
                  value={activeCard.version || "v0.1.0"}
                  onChange={(e) => updateCardVersion(activeCard.id, e.target.value)}
                  className="w-16 text-center text-xs font-mono text-purple-400 bg-[#232733] focus:bg-[#2f3545] focus:text-white focus:outline-none rounded-lg px-2 py-1 transition"
                  title="Card Version"
                />
                <button className="p-1.5 text-gray-400 hover:text-white hover:bg-[#232734] rounded-lg transition" title="AI Assist">
                  <Sparkles size={16} className="text-purple-400" />
                </button>
                <button
                  onClick={(e) => handleMoveToTrash(activeCard.id, e)}
                  className="p-1.5 text-gray-400 hover:text-rose-400 hover:bg-rose-500/20 rounded-lg transition"
                  title="Move to Trash"
                >
                  <Trash2 size={16} />
                </button>
              </div>
            </div>

            <div className="flex-1 px-6 py-6 flex flex-col gap-6">
              <div className="group/title relative flex items-start">
                <textarea
                  ref={sidebarTitleRef}
                  rows={1}
                  value={activeCard.title}
                  onChange={(e) => {
                    updateCardContent(activeCard.id, e.target.value, activeCard.content);
                    onNodeDataSync(activeCard.id, e.target.value, activeCard.content);
                    if (sidebarTitleRef.current) {
                      sidebarTitleRef.current.style.height = "0px";
                      sidebarTitleRef.current.style.height = `${sidebarTitleRef.current.scrollHeight}px`;
                    }
                  }}
                  placeholder={activeTitleDir.isRtl ? "عنوان کارت..." : "Card title..."}
                  className={`w-full bg-transparent text-2xl font-bold text-white placeholder-gray-600 focus:outline-none border-b border-[#252937] pb-3 leading-tight resize-none overflow-hidden ${
                    activeTitleDir.isRtl ? "text-right" : "text-left"
                  }`}
                  dir={activeTitleDir.isRtl ? "rtl" : "ltr"}
                />
              </div>

              <div className="group/block relative flex items-start flex-1">
                <textarea
                  ref={sidebarContentRef}
                  value={activeCard.content}
                  onChange={(e) => {
                    updateCardContent(activeCard.id, activeCard.title, e.target.value);
                    onNodeDataSync(activeCard.id, activeCard.title, e.target.value);
                  }}
                  placeholder="Start typing note..."
                  rows={16}
                  className={`w-full flex-1 bg-transparent text-base text-gray-200 placeholder-gray-600 resize-none overflow-hidden focus:outline-none leading-relaxed tracking-wide ${
                    isMajorRtl ? "text-right" : "text-left"
                  }`}
                  dir={isMajorRtl ? "rtl" : "ltr"}
                />
              </div>
            </div>
          </div>
        ) : activeGroup ? (
          <div className="flex flex-1 flex-col overflow-y-auto no-scrollbar">
            <div className="flex items-center justify-between border-b border-[#20242f] bg-[#171920] px-5 py-2.5">
              <button
                type="button"
                onClick={() => {
                  setSelectedGroupId(null);
                  setSelectedGroupVersionId(null);
                  setGroupError(null);
                }}
                className="flex items-center gap-1.5 rounded-lg bg-[#202430] px-3 py-1.5 text-sm font-medium text-gray-300 transition hover:bg-[#282d3c] hover:text-white"
              >
                <ChevronLeft size={16} />
                <span>Library</span>
              </button>
              <span className="rounded bg-violet-500/10 px-2 py-1 font-mono text-xs text-violet-300">
                {selectedGroupVersion?.version ?? activeGroup.version}
              </span>
            </div>
            <div className="flex flex-1 flex-col gap-4 px-5 py-5">
              <div className="flex items-center justify-between">
                <div>
                  <h2 className="text-lg font-semibold text-white">{activeGroup.name}</h2>
                  <p className="mt-1 text-xs text-zinc-400">
                    {displayedGroupSnapshot?.nodes.length ?? 0} items · {displayedGroupSnapshot?.edges.length ?? 0} connections
                  </p>
                </div>
                {activeGroup.deleted_at === null ? (
                  <button
                    type="button"
                    onClick={() => void handleGroupTrashAction(activeGroup.id, "trash")}
                    title="Move group to Trash"
                    aria-label="Move group to Trash"
                    className="rounded-lg p-2 text-zinc-400 transition hover:bg-rose-500/15 hover:text-rose-300"
                  >
                    <Trash2 size={16} />
                  </button>
                ) : (
                  <button
                    type="button"
                    onClick={() => void handleGroupTrashAction(activeGroup.id, "restore")}
                    title="Restore group"
                    aria-label="Restore group"
                    className="rounded-lg p-2 text-zinc-400 transition hover:bg-emerald-500/15 hover:text-emerald-300"
                  >
                    <RotateCcw size={16} />
                  </button>
                )}
              </div>
              <section className="rounded-xl border border-[#262b37] bg-[#111318] p-3">
                <h3 className="mb-2 text-xs font-semibold text-zinc-200">
                  Active Board uses ({groupUsages.length} sections in {new Set(groupUsages.map((usage) => usage.board_id)).size} Boards)
                </h3>
                {groupUsages.length === 0 ? (
                  <p className="text-xs text-zinc-500">This Library item is not used on any active Board.</p>
                ) : (
                  <ul className="max-h-40 space-y-1 overflow-y-auto">
                    {groupUsages.map((usage) => (
                      <li key={`${usage.board_id}:${usage.node_id}:${usage.segment_id}`}>
                        <button
                          type="button"
                          onClick={() => window.dispatchEvent(new CustomEvent("vora:navigate-path-usage", {
                            detail: {
                              boardId: usage.board_id,
                              nodeId: usage.node_id,
                              segmentId: usage.segment_id,
                            },
                          }))}
                          className="flex w-full items-center justify-between gap-3 rounded px-2 py-1.5 text-left text-xs text-zinc-300 hover:bg-[#202430]"
                          title="Open this Path section on its Board"
                        >
                          <span className="min-w-0 truncate">{usage.board_title} · {usage.segment_name}</span>
                          <span className="shrink-0 font-mono text-[10px] text-zinc-500">
                            ({usage.start_x.toFixed(0)}, {usage.start_y.toFixed(0)}) -&gt; ({usage.end_x.toFixed(0)}, {usage.end_y.toFixed(0)})
                          </span>
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </section>
              <div className="rounded-xl border border-[#262b37] bg-[#111318] p-3">
                <div className="mb-2 flex items-center justify-between">
                  <p className="text-[10px] font-semibold uppercase tracking-wide text-zinc-500">Complete schematic</p>
                  <span className="flex items-center gap-1 text-[10px] text-zinc-500">
                    <History size={12} />
                    Version history
                  </span>
                </div>
                {displayedGroupSnapshot && (
                  <GroupSchematicPreview
                    snapshot={displayedGroupSnapshot}
                    label={`Schematic preview of ${activeGroup.name}`}
                    className="h-52 w-full rounded-lg bg-[#0b0c0f]"
                  />
                )}
                <div className="mt-3 flex flex-col gap-1.5">
                  {groupVersions.map((version) => (
                    <button
                      key={version.id}
                      type="button"
                      aria-pressed={selectedGroupVersionId === version.id}
                      onClick={() => setSelectedGroupVersionId(version.id)}
                      className={`flex items-center justify-between rounded-md border px-2.5 py-2 text-xs transition ${
                        selectedGroupVersionId === version.id
                          ? "border-amber-500/50 bg-amber-500/10 text-amber-200"
                          : "border-transparent bg-[#191c24] text-zinc-300 hover:bg-[#22252d]"
                      }`}
                    >
                      <span className="font-mono">{version.version}</span>
                      <span className="text-[10px] text-zinc-500">
                        {new Date(version.created_at).toLocaleString()}
                        {version.version === activeGroup.version ? " · Current" : ""}
                      </span>
                    </button>
                  ))}
                </div>
              </div>
              {(groupError || itemGroupError) && (
                <p role="alert" className="text-xs text-rose-300">{groupError ?? itemGroupError}</p>
              )}
              {activeGroup.deleted_at === null && displayedGroupSnapshot && (
                <button
                  type="button"
                  onClick={() => {
                    setGroupError(null);
                    void onOpenLibraryBoard(
                      activeGroup,
                      displayedGroupSnapshot,
                      selectedGroupVersion?.version ?? activeGroup.version,
                    ).catch((cause: unknown) => {
                      console.error("Could not open Library board:", cause);
                      setGroupError(cause instanceof Error ? cause.message : String(cause));
                    });
                  }}
                  className="flex items-center justify-center gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-2.5 text-sm font-semibold text-amber-200 transition hover:bg-amber-500/20"
                >
                  <Pencil size={15} />
                  Open Library board
                </button>
              )}
              {activeGroup.deleted_at === null && (
                <button
                  type="button"
                  onClick={() => handleInsertGroup({
                    ...activeGroup,
                    version: selectedGroupVersion?.version ?? activeGroup.version,
                    snapshot: displayedGroupSnapshot ?? activeGroup.snapshot,
                  })}
                  className="mt-auto flex items-center justify-center gap-2 rounded-lg bg-violet-600 px-3 py-2.5 text-sm font-semibold text-white transition hover:bg-violet-500"
                >
                  <Plus size={16} />
                  Add copy to current Board
                </button>
              )}
            </div>
          </div>
        ) : (
          <div className="flex flex-col flex-1 overflow-hidden">
            <div className="px-5 py-3.5 flex items-center justify-between text-sm text-gray-400 relative z-40">
              <div className="flex items-center gap-2">
                <div className="relative">
                  <button
                    onClick={() => {
                      setIsFilterMenuOpen(!isFilterMenuOpen);
                      setIsSortMenuOpen(false);
                    }}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium transition bg-[#1d2028] hover:bg-[#252934] text-gray-200 border border-[#262b37]"
                  >
                    <span>
                      {currentFilter === "all" ? "All"
                        : currentFilter === "cards" ? "Cards"
                          : currentFilter === "item-groups" ? "Item Groups" : "Trash"}
                    </span>
                    <ChevronDown size={14} />
                  </button>

                  {isFilterMenuOpen && (
                    <>
                      <div
                        className="fixed inset-0 z-40"
                        onClick={() => setIsFilterMenuOpen(false)}
                      />
                      <div className="absolute top-9 left-0 w-40 bg-[#181a22] border border-[#2b303c] rounded-xl shadow-2xl py-1.5 z-50 flex flex-col text-sm">
                        <button
                          onClick={() => { setCurrentFilter("all"); setSelectedCardId(null); setSelectedGroupId(null); setIsFilterMenuOpen(false); }}
                          className={`flex items-center gap-2.5 px-3.5 py-2 transition text-left font-medium ${
                            currentFilter === "all" ? "bg-[#252a37] text-white" : "hover:bg-[#222734] text-gray-300 hover:text-white"
                          }`}
                        >
                          <Archive size={14} />
                          <span>All</span>
                        </button>
                        <button
                          onClick={() => { setCurrentFilter("cards"); setSelectedCardId(null); setSelectedGroupId(null); setIsFilterMenuOpen(false); }}
                          className={`flex items-center gap-2.5 px-3.5 py-2 transition text-left font-medium ${
                            currentFilter === "cards" ? "bg-[#252a37] text-white" : "hover:bg-[#222734] text-gray-300 hover:text-white"
                          }`}
                        >
                          <FileText size={14} />
                          <span>Cards</span>
                        </button>
                        <button
                          onClick={() => { setCurrentFilter("item-groups"); setSelectedCardId(null); setSelectedGroupId(null); setIsFilterMenuOpen(false); }}
                          className={`flex items-center gap-2.5 px-3.5 py-2 transition text-left font-medium ${
                            currentFilter === "item-groups" ? "bg-[#252a37] text-white" : "hover:bg-[#222734] text-gray-300 hover:text-white"
                          }`}
                        >
                          <Layers size={14} />
                          <span>Item Groups</span>
                        </button>
                        <button
                          onClick={() => { setCurrentFilter("trash"); setSelectedCardId(null); setSelectedGroupId(null); setIsFilterMenuOpen(false); }}
                          className={`flex items-center gap-2.5 px-3.5 py-2 transition text-left font-medium ${
                            currentFilter === "trash" ? "bg-[#252a37] text-white" : "hover:bg-[#222734] text-gray-300 hover:text-white"
                          }`}
                        >
                          <Trash2 size={14} />
                          <span>Trash</span>
                        </button>
                      </div>
                    </>
                  )}
                </div>

                <button className="flex items-center gap-1.5 px-3 py-1.5 bg-[#1d2028] hover:bg-[#252934] rounded-lg text-sm font-medium text-gray-200 border border-[#262b37] transition">
                  <Tag size={14} />
                  <span>Tags</span>
                  <ChevronDown size={14} />
                </button>
              </div>

              <div className="flex items-center gap-1.5">
                <button className="p-2 bg-[#1d2028] hover:bg-[#252934] rounded-lg text-gray-300 border border-[#262b37] transition" title="Filter">
                  <Filter size={15} />
                </button>

                <div className="relative">
                  <button
                    onClick={() => {
                      setIsSortMenuOpen(!isSortMenuOpen);
                      setIsFilterMenuOpen(false);
                    }}
                    className="p-2 rounded-lg border transition bg-[#1d2028] hover:bg-[#252934] text-gray-300 border-[#262b37]"
                    title="Sort & Ordering"
                  >
                    <ArrowUpDown size={15} />
                  </button>

                  {isSortMenuOpen && (
                    <>
                      <div
                        className="fixed inset-0 z-40"
                        onClick={() => setIsSortMenuOpen(false)}
                      />
                      <div className="absolute top-9 right-0 w-44 bg-[#181a22] border border-[#282d3b] rounded-xl shadow-2xl py-2 px-1.5 z-50 flex flex-col text-xs text-gray-200">
                        <span className="px-2.5 py-1 text-[11px] font-semibold text-white tracking-wider">
                          SORT BY
                        </span>

                        <button
                          onClick={() => {
                            setSortBy("alphabetical");
                            setIsSortMenuOpen(false);
                          }}
                          className="w-full flex items-center justify-between px-2.5 py-1.5 transition rounded-lg text-gray-200 hover:bg-[#242834] hover:text-white text-left"
                        >
                          <span>Alphabetical</span>
                          {sortBy === "alphabetical" && (
                            <div className="w-1.5 h-1.5 rounded-full bg-purple-400 flex-shrink-0" />
                          )}
                        </button>

                        <button
                          onClick={() => {
                            setSortBy("created");
                            setIsSortMenuOpen(false);
                          }}
                          className="w-full flex items-center justify-between px-2.5 py-1.5 transition rounded-lg text-gray-200 hover:bg-[#242834] hover:text-white text-left"
                        >
                          <span>Date created</span>
                          {sortBy === "created" && (
                            <div className="w-1.5 h-1.5 rounded-full bg-purple-400 flex-shrink-0" />
                          )}
                        </button>

                        <button
                          onClick={() => {
                            setSortBy("viewed");
                            setIsSortMenuOpen(false);
                          }}
                          className="w-full flex items-center justify-between px-2.5 py-1.5 transition rounded-lg text-gray-200 hover:bg-[#242834] hover:text-white text-left"
                        >
                          <span>Last viewed</span>
                          {sortBy === "viewed" && (
                            <div className="w-1.5 h-1.5 rounded-full bg-purple-400 flex-shrink-0" />
                          )}
                        </button>

                        <div className="h-[1px] bg-[#282d3b] my-1.5 mx-1" />

                        <span className="px-2.5 py-1 text-[11px] font-semibold text-white tracking-wider">
                          ORDER
                        </span>

                        <button
                          onClick={() => {
                            setSortOrder("oldest");
                            setIsSortMenuOpen(false);
                          }}
                          className="w-full flex items-center justify-between px-2.5 py-1.5 transition rounded-lg text-gray-200 hover:bg-[#242834] hover:text-white text-left"
                        >
                          <span>{sortBy === "alphabetical" ? "A to Z" : "Oldest first"}</span>
                          {sortOrder === "oldest" && (
                            <div className="w-1.5 h-1.5 rounded-full bg-purple-400 flex-shrink-0" />
                          )}
                        </button>

                        <button
                          onClick={() => {
                            setSortOrder("newest");
                            setIsSortMenuOpen(false);
                          }}
                          className="w-full flex items-center justify-between px-2.5 py-1.5 transition rounded-lg text-gray-200 hover:bg-[#242834] hover:text-white text-left"
                        >
                          <span>{sortBy === "alphabetical" ? "Z to A" : "Newest first"}</span>
                          {sortOrder === "newest" && (
                            <div className="w-1.5 h-1.5 rounded-full bg-purple-400 flex-shrink-0" />
                          )}
                        </button>
                      </div>
                    </>
                  )}
                </div>
              </div>
            </div>

            <div className="px-5 mb-4 relative z-10">
              <div className="relative flex items-center w-full">
                <Search size={16} className="absolute left-3.5 text-gray-500 pointer-events-none" />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search library..."
                  className="w-full bg-[#0f1115] border border-[#232733] focus:border-purple-500/60 rounded-xl py-2 pl-10 pr-3.5 text-sm text-gray-100 placeholder-gray-500 focus:outline-none transition"
                />
              </div>
            </div>

            <div className="flex-1 overflow-y-auto no-scrollbar px-5 pb-5 grid grid-cols-2 gap-3.5 content-start">
              {itemGroupError && <p role="alert" className="col-span-2 text-xs text-rose-300">{itemGroupError}</p>}
              {(currentFilter === "all" || currentFilter === "cards") && (
                <button
                  onClick={onAddNewCard}
                  className={`h-52 rounded-2xl bg-[#191c24] hover:bg-[#20242f] border border-[#262b37] hover:border-purple-500/50 flex flex-col items-center justify-center gap-3 text-gray-200 hover:text-white transition-all duration-150 shadow-sm group ${
                    isLibraryEmpty ? "col-span-2 w-52 mx-auto" : "w-full"
                  }`}
                >
                  <div className="p-3 rounded-2xl bg-[#222733] group-hover:scale-110 transition-transform">
                    <Plus size={22} className="text-purple-400" />
                  </div>
                  <span className="text-sm font-semibold tracking-wide">New card</span>
                </button>
              )}

              {filteredCards.map((c) => {
                const itemTitleDir = resolveTextDirection(c.title || "", isMajorRtl);
                const mediaPreview = resolveLibraryMediaPreview(c.content);
                return (
                  <div
                    key={c.id}
                    onPointerDown={(e) => {
                      if (c.deleted_at == null && currentFilter !== "trash") {
                        handleLibraryItemPointerDown(e, { kind: "card", card: c });
                      }
                    }}
                    onClick={(event) => {
                      if (!handleLibraryItemClick(event, "card", c.id)) setSelectedCardId(c.id);
                    }}
                    className="group relative h-52 p-3.5 rounded-2xl bg-[#191c24] hover:bg-[#20242f] border border-[#262b37] hover:border-purple-500/50 cursor-grab active:cursor-grabbing transition-all duration-150 flex flex-col justify-between shadow-sm"
                  >
                    <div className="flex items-center justify-between min-h-[22px] pb-1 border-b border-[#232836]/40 pointer-events-auto">
                      <span className="text-[10px] font-mono text-purple-400/80 px-1.5 py-0.5 bg-purple-500/10 rounded">
                        {c.version || "v0.1.0"}
                      </span>

                      {currentFilter !== "trash" ? (
                        <button
                          onClick={(e) => handleMoveToTrash(c.id, e)}
                          className="opacity-0 group-hover:opacity-100 p-1 rounded-md hover:bg-rose-500/20 text-gray-400 hover:text-rose-400 transition"
                          title="Move to Trash"
                        >
                          <Trash2 size={14} />
                        </button>
                      ) : (
                        <button
                          onClick={(e) => handleRestore(c.id, e)}
                          className="opacity-0 group-hover:opacity-100 p-1 rounded-md hover:bg-emerald-500/20 text-gray-400 hover:text-emerald-400 transition"
                          title="Restore Card"
                        >
                          <RotateCcw size={14} />
                        </button>
                      )}
                    </div>

                    <div className="flex items-center gap-2 pt-2 text-gray-400 pointer-events-none">
                      <FileText size={14} className="text-purple-400 flex-shrink-0" />
                      <span
                        className={`text-sm font-bold text-gray-100 truncate w-full ${
                          itemTitleDir.isRtl ? "text-right" : "text-left"
                        }`}
                        dir={itemTitleDir.isRtl ? "rtl" : "ltr"}
                      >
                        {itemTitleDir.cleanText || "Untitled"}
                      </span>
                    </div>

                    <div className={`flex-1 overflow-hidden border-t border-[#232836] pt-2 pr-0.5 flex flex-col gap-0.5 ${
                      mediaPreview ? "justify-center" : "pointer-events-none"
                    }`}>
                      {mediaPreview ? (
                        <LibraryMediaPreview media={mediaPreview} />
                      ) : c.content ? (
                        c.content.split("\n").slice(0, 5).map((line, lIdx) => {
                          const lineDir = resolveTextDirection(line, isMajorRtl);
                          return (
                            <p
                              key={lIdx}
                              className={`text-sm text-gray-300 truncate ${
                                lineDir.isRtl ? "text-right" : "text-left"
                              }`}
                              dir={lineDir.isRtl ? "rtl" : "ltr"}
                            >
                              {lineDir.cleanText || "\u00A0"}
                            </p>
                          );
                        })
                      ) : (
                        <p className="text-sm text-gray-500">Notes & content...</p>
                      )}
                    </div>
                  </div>
                );
              })}

              {filteredGroups.map((group) => {
                const groupTitleDir = resolveTextDirection(group.name, isMajorRtl);
                return (
                  <article
                    key={`group-${group.id}`}
                    onPointerDown={(event) => {
                      if (group.deleted_at === null && group.snapshot.nodes.length > 0) {
                        handleLibraryItemPointerDown(event, { kind: "group", group });
                      }
                    }}
                    title={group.deleted_at === null ? "Drag schematic to the board or click to preview" : undefined}
                    className={`group relative flex h-52 flex-col rounded-2xl border border-[#34302a] bg-[#1c1b19] p-3.5 shadow-sm transition-all duration-150 hover:border-amber-500/50 hover:bg-[#24211c] ${
                      group.deleted_at === null && group.snapshot.nodes.length > 0
                        ? "cursor-grab active:cursor-grabbing"
                        : ""
                    }`}
                  >
                    <button
                      type="button"
                      onClick={(event) => {
                        if (handleLibraryItemClick(event, "group", group.id)) return;
                        setSelectedCardId(null);
                        setSelectedGroupId(group.id);
                        setGroupError(null);
                      }}
                      className="absolute inset-0 z-0 rounded-2xl focus-visible:outline focus-visible:outline-2 focus-visible:outline-amber-400"
                      aria-label={`Open item group ${group.name}`}
                      title="Open item group preview"
                    />
                    <div className="relative z-10 flex min-h-0 flex-1 flex-col gap-2 pointer-events-none">
                      <div className="flex items-center justify-between border-b border-[#302b24] pb-1.5">
                        <span className="rounded bg-amber-500/10 px-1.5 py-0.5 font-mono text-[10px] text-amber-300">
                          {group.version}
                        </span>
                        <div className="flex items-center gap-1">
                          <span className="text-[9px] text-zinc-500">
                            {group.snapshot.nodes.length} items
                          </span>
                          {group.deleted_at === null ? (
                            <button
                              type="button"
                              onClick={(event) => {
                                event.stopPropagation();
                                void handleGroupTrashAction(group.id, "trash");
                              }}
                              title="Move to Trash"
                              aria-label={`Move ${group.name} to Trash`}
                              className="pointer-events-auto rounded p-1 text-zinc-400 transition hover:bg-rose-500/20 hover:text-rose-300"
                            >
                              <Trash2 size={13} />
                            </button>
                          ) : (
                            <button
                              type="button"
                              onClick={(event) => {
                                event.stopPropagation();
                                void handleGroupTrashAction(group.id, "restore");
                              }}
                              title="Restore item group"
                              aria-label={`Restore ${group.name}`}
                              className="pointer-events-auto rounded p-1 text-zinc-400 transition hover:bg-emerald-500/20 hover:text-emerald-300"
                            >
                              <RotateCcw size={13} />
                            </button>
                          )}
                        </div>
                      </div>
                      <div
                        className={`flex items-center gap-2 text-sm font-semibold text-zinc-100 ${
                          groupTitleDir.isRtl ? "text-right" : "text-left"
                        }`}
                        dir={groupTitleDir.isRtl ? "rtl" : "ltr"}
                      >
                        <Layers size={14} className="shrink-0 text-amber-300" />
                        <span className="truncate">{groupTitleDir.cleanText || "Untitled"}</span>
                      </div>
                      <div className="flex min-h-0 flex-1 flex-col justify-center gap-1.5 overflow-hidden border-t border-[#302b24] pt-1.5">
                        <GroupSchematicPreview
                          snapshot={group.snapshot}
                          label={`Complete schematic preview of ${group.name}`}
                          className="h-full min-h-24 w-full rounded-md bg-[#141311]"
                        />
                      </div>
                    </div>
                  </article>
                );
              })}
              {groupError && <p role="alert" className="col-span-2 text-xs text-rose-300">{groupError}</p>}
              {isLibraryEmpty && (
                <p className="col-span-2 py-8 text-center text-xs text-zinc-500">
                  {currentFilter === "cards" ? "No cards found."
                    : currentFilter === "item-groups" ? "No item groups found."
                      : currentFilter === "trash" ? "Trash is empty." : "Library is empty."}
                </p>
              )}
            </div>
          </div>
        )}
      </aside>
    </>
  );
};