import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactElement } from "react";
import { createPortal } from "react-dom";
import { Search, X } from "lucide-react";
import { dbService, type BoardDto, type CardDto } from "@/mind/services/dbService";
import type { TextLink, TextLinkField } from "@/mind/types";

interface TextLinkPickerProps {
  sourceBoardId: string;
  sourceCardId: string;
  sourceField: TextLinkField;
  sourceStart: number;
  sourceEnd: number;
  sourceText: string;
  onClose: () => void;
  onCreated: (link: TextLink) => void;
}

interface BoardCards {
  board: BoardDto;
  cards: CardDto[];
}

type PickerTab = "projects" | "library";

function parseTextLinkField(value: string): TextLinkField | null {
  return value === "title" || value === "content" ? value : null;
}

export function TextLinkPicker({
  sourceBoardId,
  sourceCardId,
  sourceField,
  sourceStart,
  sourceEnd,
  sourceText,
  onClose,
  onCreated,
}: TextLinkPickerProps): ReactElement {
  const [boardCards, setBoardCards] = useState<BoardCards[]>([]);
  const [boardId, setBoardId] = useState(sourceBoardId);
  const [cardId, setCardId] = useState("");
  const [field, setField] = useState<TextLinkField>("content");
  const [targetRange, setTargetRange] = useState<{ start: number; end: number } | null>(null);
  const [activeTab, setActiveTab] = useState<PickerTab>("projects");
  const [search, setSearch] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const dialogRef = useRef<HTMLElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const previouslyFocused = document.activeElement;
    const previousBodyOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    searchRef.current?.focus();

    return () => {
      document.body.style.overflow = previousBodyOverflow;
      if (previouslyFocused instanceof HTMLElement) previouslyFocused.focus();
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    const loadDestinations = async (): Promise<void> => {
      try {
        const boards = await dbService.getAllBoards();
        const loaded = await Promise.all(boards.map(async (board) => ({
          board,
          cards: (await dbService.getAllCards(board.id)).filter((card) => card.deleted_at === null),
        })));
        if (cancelled) return;
        setBoardCards(loaded);
        const initialBoard = loaded.find((item) => item.board.id === sourceBoardId) ?? loaded[0];
        if (initialBoard) {
          setBoardId(initialBoard.board.id);
          setCardId(initialBoard.cards[0]?.id ?? "");
        }
        setIsLoading(false);
      } catch (loadError) {
        if (!cancelled) {
          setError(loadError instanceof Error ? loadError.message : String(loadError));
          setIsLoading(false);
        }
      }
    };
    void loadDestinations();
    return () => { cancelled = true; };
  }, [sourceBoardId]);

  const selectedBoard = boardCards.find((item) => item.board.id === boardId);
  const targetCard = selectedBoard?.cards.find((card) => card.id === cardId);
  const targetText = targetCard?.[field] ?? "";
  const libraryCards = useMemo(() => boardCards.flatMap(({ board, cards }) =>
    cards.map((card) => ({ board, card }))
  ), [boardCards]);
  const matchingCards = libraryCards.filter(({ board, card }) => {
    const query = search.trim().toLocaleLowerCase();
    return !query || `${board.title} ${card.title} ${card.content}`.toLocaleLowerCase().includes(query);
  });
  const matchingProjectCards = (selectedBoard?.cards ?? []).filter((card) => {
    const query = search.trim().toLocaleLowerCase();
    return !query || `${card.title} ${card.content}`.toLocaleLowerCase().includes(query);
  });

  const saveLink = async (): Promise<void> => {
    if (!targetCard || sourceStart < 0 || sourceEnd <= sourceStart) {
      setError("Select a non-empty source range and a destination card.");
      return;
    }
    const range = targetRange ?? { start: 0, end: targetText.length };
    if (range.end <= range.start) {
      setError("The destination field is empty. Select a non-empty destination range.");
      return;
    }
    setError(null);
    setIsSaving(true);
    try {
      const link = await dbService.createTextLink({
        source_board_id: sourceBoardId,
        source_card_id: sourceCardId,
        source_field: sourceField,
        source_start: sourceStart,
        source_end: sourceEnd,
        target_board_id: boardId,
        target_card_id: cardId,
        target_field: field,
        target_start: range.start,
        target_end: range.end,
      });
      onCreated(link);
      onClose();
    } catch (saveError) {
      setError(saveError instanceof Error ? saveError.message : String(saveError));
    } finally {
      setIsSaving(false);
    }
  };

  const cardButtonClass = (isSelected: boolean): string =>
    `w-full rounded-lg border p-3 text-left transition ${
      isSelected
        ? "border-violet-500 bg-violet-500/10"
        : "border-[#303440] bg-[#191b20] hover:border-[#50596b] hover:bg-[#20232a]"
    }`;

  return createPortal(
    <div
      data-text-link-picker
      className="fixed inset-0 z-[100] flex h-dvh w-screen items-center justify-center overflow-hidden bg-black/65 p-4 backdrop-blur-[1px]"
      role="presentation"
      onPointerDown={(event) => {
        event.stopPropagation();
      }}
      onWheel={(event) => event.stopPropagation()}
    >
      <section
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="text-link-picker-title"
        className="flex h-[min(82dvh,700px)] w-[min(90vw,850px)] max-h-full max-w-full flex-col overflow-hidden rounded-lg border border-[#303440] bg-[#17191e] text-gray-100 shadow-2xl"
        dir="auto"
        onKeyDown={(event) => {
          event.stopPropagation();
          if (event.key === "Escape") {
            event.preventDefault();
            onClose();
            return;
          }
          if (event.key !== "Tab") return;

          const focusableElements = dialogRef.current?.querySelectorAll<HTMLElement>(
            'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])',
          );
          if (!focusableElements?.length) {
            event.preventDefault();
            return;
          }

          const firstElement = focusableElements[0];
          const lastElement = focusableElements[focusableElements.length - 1];
          if (event.shiftKey && document.activeElement === firstElement) {
            event.preventDefault();
            lastElement?.focus();
          } else if (!event.shiftKey && document.activeElement === lastElement) {
            event.preventDefault();
            firstElement?.focus();
          }
        }}
      >
        <header className="flex shrink-0 items-center justify-between border-b border-[#292d35] px-5 py-4">
          <h2 id="text-link-picker-title" className="text-sm font-semibold">Link</h2>
          <button type="button" onClick={onClose} aria-label="Close link picker" className="rounded-md p-1.5 text-gray-400 hover:bg-[#292d35] hover:text-white">
            <X size={18} />
          </button>
        </header>

        <div className="shrink-0 px-5 pt-4">
          <label className="relative block">
            <Search size={16} className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-gray-500" />
            <input
              ref={searchRef}
              type="search"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Search projects or cards..."
              aria-label="Search projects or cards"
              className="w-full rounded-lg border border-[#343946] bg-[#24262c] py-2.5 pl-10 pr-3 text-sm text-gray-100 outline-none placeholder:text-gray-500 focus:border-violet-500"
            />
          </label>
          <p className="mt-2 truncate text-xs text-gray-500" title={sourceText.slice(sourceStart, sourceEnd)}>
            Linking: <span className="text-gray-300">{sourceText.slice(sourceStart, sourceEnd)}</span>
          </p>
          <div className="mt-4 flex gap-6 border-b border-[#292d35]" role="tablist" aria-label="Link destination type">
            {(["projects", "library"] as const).map((tab) => (
              <button
                key={tab}
                type="button"
                role="tab"
                aria-selected={activeTab === tab}
                onClick={() => setActiveTab(tab)}
                className={`border-b-2 px-1 pb-2 text-sm transition ${
                  activeTab === tab
                    ? "border-blue-500 text-blue-400"
                    : "border-transparent text-gray-400 hover:text-gray-200"
                }`}
              >
                {tab === "projects" ? "Projects" : "Library"}
              </button>
            ))}
          </div>
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
          {isLoading ? (
            <p className="py-10 text-center text-sm text-gray-400">Loading cards...</p>
          ) : activeTab === "projects" ? (
            <div className="grid gap-4 sm:grid-cols-[minmax(170px,0.8fr)_minmax(0,2fr)]">
              <div className="space-y-2">
                <p className="px-1 text-xs font-medium uppercase tracking-wide text-gray-500">Projects</p>
                {boardCards.map(({ board, cards }) => (
                  <button
                    key={board.id}
                    type="button"
                    onClick={() => {
                      setBoardId(board.id);
                      setCardId(cards[0]?.id ?? "");
                      setTargetRange(null);
                    }}
                    className={`w-full rounded-lg px-3 py-2.5 text-left text-sm transition ${
                      boardId === board.id ? "bg-[#252a34] text-white" : "text-gray-300 hover:bg-[#20232a]"
                    }`}
                  >
                    <span className="block truncate font-medium">{board.title}</span>
                    <span className="mt-0.5 block text-xs text-gray-500">{cards.length} cards</span>
                  </button>
                ))}
              </div>
              <div className="space-y-2">
                <p className="px-1 text-xs font-medium uppercase tracking-wide text-gray-500">Cards</p>
                {matchingProjectCards.length === 0 ? (
                  <p className="rounded-lg border border-dashed border-[#343946] p-5 text-center text-sm text-gray-500">
                    {search ? "No matching cards." : "This project has no active cards."}
                  </p>
                ) : (
                  <div className="grid gap-2 sm:grid-cols-2">
                    {matchingProjectCards.map((card) => (
                      <button
                        key={card.id}
                        type="button"
                        onClick={() => {
                          setCardId(card.id);
                          setTargetRange(null);
                        }}
                        className={cardButtonClass(cardId === card.id)}
                      >
                        <span className="block truncate text-sm font-medium">{card.title || "Untitled card"}</span>
                        <span className="mt-1 line-clamp-2 block text-xs leading-relaxed text-gray-500">
                          {card.content || "No description"}
                        </span>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </div>
          ) : matchingCards.length === 0 ? (
            <p className="rounded-lg border border-dashed border-[#343946] p-10 text-center text-sm text-gray-500">
              {search ? "No matching cards." : "There are no active cards in the Library yet."}
            </p>
          ) : (
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {matchingCards.map(({ board, card }) => (
                <button
                  key={`${board.id}:${card.id}`}
                  type="button"
                  onClick={() => {
                    setBoardId(board.id);
                    setCardId(card.id);
                    setTargetRange(null);
                  }}
                  className={cardButtonClass(boardId === board.id && cardId === card.id)}
                >
                  <span className="block text-[11px] text-gray-500">{board.title}</span>
                  <span className="mt-1 block truncate text-sm font-semibold">{card.title || "Untitled card"}</span>
                  <span className="mt-2 line-clamp-4 min-h-12 text-xs leading-relaxed text-gray-400">
                    {card.content || "No description"}
                  </span>
                </button>
              ))}
            </div>
          )}

          {targetCard && (
            <div className="mt-5 border-t border-[#292d35] pt-4">
              <div className="mb-3 flex flex-wrap items-end justify-between gap-3">
                <div className="min-w-0">
                  <p className="text-xs text-gray-500">Destination</p>
                  <p className="truncate text-sm font-medium">{targetCard.title || "Untitled card"}</p>
                </div>
                <label className="space-y-1 text-xs text-gray-400">
                  Destination field
                  <select
                    value={field}
                    onChange={(event) => {
                      const selectedField = parseTextLinkField(event.target.value);
                      if (selectedField) setField(selectedField);
                      setTargetRange(null);
                    }}
                    className="ml-2 rounded-md border border-[#343946] bg-[#20232a] px-3 py-2 text-sm text-gray-100 outline-none focus:border-violet-500"
                  >
                    <option value="title">Title</option>
                    <option value="content">Description</option>
                  </select>
                </label>
              </div>
              <label htmlFor="text-link-target" className="mb-1 block text-xs text-gray-500">
                Select a target phrase, or leave it unselected to link the whole field.
              </label>
              <textarea
                id="text-link-target"
                readOnly
                value={targetText}
                onSelect={(event) => {
                  const start = event.currentTarget.selectionStart;
                  const end = event.currentTarget.selectionEnd;
                  setTargetRange(end > start ? { start, end } : null);
                }}
                onMouseUp={(event) => {
                  const start = event.currentTarget.selectionStart;
                  const end = event.currentTarget.selectionEnd;
                  setTargetRange(end > start ? { start, end } : null);
                }}
                dir="auto"
                className="max-h-32 min-h-20 w-full resize-none rounded-lg border border-[#343946] bg-[#111318] p-3 text-sm leading-relaxed text-gray-200 outline-none focus:border-violet-500"
              />
            </div>
          )}
        </div>

        <footer className="grid shrink-0 grid-cols-[1fr_auto_1fr] items-center gap-3 border-t border-[#292d35] bg-[#14161a] px-5 py-4">
          {error ? (
            <p role="alert" className="text-sm text-rose-400">{error}</p>
          ) : (
            <span className="text-xs text-gray-500">
              {targetRange ? `${targetRange.end - targetRange.start} characters selected` : "Whole destination field"}
            </span>
          )}
          <button
            type="button"
            onClick={() => void saveLink()}
            disabled={isSaving || isLoading || !targetCard || targetText.length === 0}
            className="min-w-36 rounded-lg bg-blue-600 px-5 py-2.5 text-sm font-semibold text-white shadow-sm transition hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {isSaving ? "Creating..." : "Confirm"}
          </button>
          <span aria-hidden="true" />
        </footer>
      </section>
    </div>,
    document.body,
  );
}
