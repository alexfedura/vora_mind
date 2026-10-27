import React, { useEffect, useMemo, useState } from "react";
import { CalendarDays, ChevronLeft, ChevronRight, Clock3, House, Menu } from "lucide-react";
import { useAppStore } from "@/shared/store/useAppStore";
import { MarkdownTextEditor } from "@/features/markdown-editor/MarkdownTextEditor";

function dateKey(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function dateFromKey(value: string): Date {
  const [year, month, day] = value.split("-").map(Number);
  return new Date(year ?? 0, (month ?? 1) - 1, day ?? 1);
}

function shiftDate(date: Date, days: number): Date {
  const next = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  next.setDate(next.getDate() + days);
  return next;
}

const fullDateFormatter = new Intl.DateTimeFormat("en-US", {
  weekday: "long",
  month: "long",
  day: "numeric",
  year: "numeric",
});

const dateHeadingFormatter = new Intl.DateTimeFormat("en-US", {
  month: "short",
  day: "numeric",
  year: "numeric",
});

const monthFormatter = new Intl.DateTimeFormat("en-US", {
  month: "long",
  year: "numeric",
});

const weekdayFormatter = new Intl.DateTimeFormat("en-US", { weekday: "long" });

function getMonthBounds(date: Date): [string, string] {
  const firstDay = new Date(date.getFullYear(), date.getMonth(), 1);
  const lastDay = new Date(date.getFullYear(), date.getMonth() + 1, 0);
  return [dateKey(firstDay), dateKey(lastDay)];
}

interface JournalPageProps {
  onGoHome: () => void;
}

export const JournalPage: React.FC<JournalPageProps> = ({ onGoHome }) => {
  const entries = useAppStore((state) => state.journalEntries);
  const saveStatus = useAppStore((state) => state.journalSaveStatus);
  const saveError = useAppStore((state) => state.journalSaveError);
  const loadEntries = useAppStore((state) => state.loadJournalEntries);
  const updateContent = useAppStore((state) => state.updateJournalEntryContent);
  const flushEntry = useAppStore((state) => state.flushJournalEntry);
  const [selectedDate, setSelectedDate] = useState(() => dateKey(new Date()));
  const [isMonthView, setIsMonthView] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [isHomeMenuOpen, setIsHomeMenuOpen] = useState(false);
  const selectedDay = useMemo(() => dateFromKey(selectedDate), [selectedDate]);
  const [monthStart, monthEnd] = getMonthBounds(selectedDay);
  const daysInMonth = new Date(selectedDay.getFullYear(), selectedDay.getMonth() + 1, 0).getDate();
  const entryForSelectedDate = entries[selectedDate];
  const currentSaveStatus = saveStatus[selectedDate];

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setLoadError(null);
    void loadEntries(monthStart, monthEnd)
      .catch((error: unknown) => {
        console.error("Could not load Journal entries:", error);
        if (!cancelled) {
          setLoadError(error instanceof Error ? error.message : String(error));
        }
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [loadEntries, monthEnd, monthStart]);

  const moveSelection = (days: number): void => {
    setSelectedDate(dateKey(shiftDate(selectedDay, days)));
  };

  const moveMonth = (months: number): void => {
    const targetMonth = new Date(selectedDay.getFullYear(), selectedDay.getMonth() + months, 1);
    const targetDay = Math.min(
      selectedDay.getDate(),
      new Date(targetMonth.getFullYear(), targetMonth.getMonth() + 1, 0).getDate(),
    );
    setSelectedDate(dateKey(new Date(targetMonth.getFullYear(), targetMonth.getMonth(), targetDay)));
  };

  const handleBlur = (): void => {
    void flushEntry(selectedDate).catch(() => undefined);
  };

  return (
    <section className="absolute inset-0 z-20 flex min-h-0 flex-col overflow-hidden bg-[#101114] text-gray-200">
      <header className="border-b border-[#292c34] bg-[#14161b] px-6 py-5 sm:px-10">
        <div className="mx-auto flex w-full max-w-6xl flex-wrap items-center justify-between gap-4">
          <div className="pl-12 sm:pl-14">
            <h1 className="text-2xl font-semibold tracking-tight text-white">
              {isMonthView
                ? monthFormatter.format(selectedDay)
                : dateHeadingFormatter.format(selectedDay)}
            </h1>
            <p className="mt-1 flex items-center gap-1.5 text-sm text-purple-300">
              <Clock3 size={14} />
              {isMonthView
                ? "Monthly archive"
                : fullDateFormatter.format(selectedDay).split(",")[0]}
            </p>
          </div>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={() => setIsMonthView((value) => !value)}
              aria-pressed={isMonthView}
              className={`flex items-center gap-2 rounded-xl border px-3 py-2 text-xs font-semibold transition ${
                isMonthView
                  ? "border-purple-500/40 bg-purple-500/15 text-purple-200"
                  : "border-[#343741] bg-[#1a1c22] text-gray-300 hover:border-purple-500/40 hover:text-white"
              }`}
            >
              <CalendarDays size={15} />
              {isMonthView ? "Daily" : "Month"}
            </button>
            {isMonthView ? (
              <>
                <button
                  type="button"
                  onClick={() => moveMonth(-1)}
                  aria-label="Previous month"
                  className="rounded-lg border border-[#343741] bg-[#1a1c22] p-2 text-gray-300 transition hover:border-purple-500/40 hover:text-white"
                >
                  <ChevronLeft size={16} />
                </button>
                <button
                  type="button"
                  onClick={() => moveMonth(1)}
                  aria-label="Next month"
                  className="rounded-lg border border-[#343741] bg-[#1a1c22] p-2 text-gray-300 transition hover:border-purple-500/40 hover:text-white"
                >
                  <ChevronRight size={16} />
                </button>
              </>
            ) : (
              <>
                <button
                  type="button"
                  onClick={() => moveSelection(-1)}
                  aria-label="Previous day"
                  className="rounded-lg border border-[#343741] bg-[#1a1c22] p-2 text-gray-300 transition hover:border-purple-500/40 hover:text-white"
                >
                  <ChevronLeft size={16} />
                </button>
                <button
                  type="button"
                  onClick={() => moveSelection(1)}
                  aria-label="Next day"
                  className="rounded-lg border border-[#343741] bg-[#1a1c22] p-2 text-gray-300 transition hover:border-purple-500/40 hover:text-white"
                >
                  <ChevronRight size={16} />
                </button>
              </>
            )}
            <button
              type="button"
              onClick={() => {
                setSelectedDate(dateKey(new Date()));
                setIsMonthView(false);
              }}
              aria-label="Open today's journal entry"
              className="rounded-xl border border-[#343741] bg-[#1a1c22] px-3 py-2 text-xs font-semibold text-gray-300 transition hover:border-purple-500/40 hover:text-white"
            >
              Today
            </button>
            <div
              className="group relative"
              onMouseEnter={() => setIsHomeMenuOpen(true)}
              onMouseLeave={() => setIsHomeMenuOpen(false)}
              onFocusCapture={() => setIsHomeMenuOpen(true)}
              onBlurCapture={(event) => {
                if (!event.currentTarget.contains(event.relatedTarget)) {
                  setIsHomeMenuOpen(false);
                }
              }}
            >
              <button
                type="button"
                onClick={onGoHome}
                aria-label="Go to Home"
                aria-haspopup="menu"
                aria-expanded={isHomeMenuOpen}
                title="Home"
                className="flex h-9 w-9 items-center justify-center rounded-xl border border-[#343741] bg-[#1a1c22] text-gray-300 transition hover:border-purple-500/40 hover:bg-purple-500/10 hover:text-white"
              >
                {isHomeMenuOpen
                  ? <Menu size={17} aria-hidden="true" />
                  : <House size={17} aria-hidden="true" />}
              </button>
              <div
                role="menu"
                aria-label="Journal navigation"
                className={`absolute right-0 top-full z-50 mt-1 min-w-[11rem] rounded-xl border border-[#2a2a2a] bg-[#181818]/90 p-1.5 shadow-lg backdrop-blur transition-all duration-200 ${
                  isHomeMenuOpen ? "visible translate-y-0 opacity-100" : "invisible -translate-y-1 opacity-0"
                }`}
              >
                <button
                  type="button"
                  role="menuitem"
                  onClick={() => {
                    setIsMonthView(false);
                    setIsHomeMenuOpen(false);
                  }}
                  className="flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-left text-sm font-medium text-gray-200 transition hover:bg-[#1c202d] hover:text-violet-300"
                >
                  <BookOpen size={15} aria-hidden="true" />
                  <span>Journal</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  onClick={() => {
                    setIsHomeMenuOpen(false);
                    onGoHome();
                  }}
                  className="flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-left text-sm font-medium text-gray-200 transition hover:bg-[#1c202d] hover:text-violet-300"
                >
                  <House size={15} aria-hidden="true" />
                  <span>Home</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto px-6 pt-[30px] pb-5 sm:px-10">
        <div className="mx-auto w-full max-w-6xl">
          {loadError && (
            <div role="alert" className="mb-4 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-rose-900/60 bg-rose-950/30 px-4 py-3 text-sm text-rose-200">
              <span>Could not load journal entries: {loadError}</span>
              <button
                type="button"
                onClick={() => {
                  setLoadError(null);
                  setIsLoading(true);
                  void loadEntries(monthStart, monthEnd)
                    .catch((error: unknown) => {
                      console.error("Could not reload Journal entries:", error);
                      setLoadError(error instanceof Error ? error.message : String(error));
                    })
                    .finally(() => setIsLoading(false));
                }}
                className="font-semibold underline underline-offset-2"
              >
                Retry
              </button>
            </div>
          )}

          {isMonthView ? (
            <>
              <div className="mb-5 flex items-end justify-between gap-4">
                <div>
                  <p className="text-xs font-semibold uppercase tracking-[0.16em] text-purple-300">Monthly archive</p>
                  <h2 className="mt-1 text-2xl font-semibold text-white">
                    {monthFormatter.format(selectedDay)}
                  </h2>
                </div>
                <p className="text-xs text-gray-500">Select a date to open its journal</p>
              </div>

              {isLoading ? (
                <p className="py-12 text-center text-sm text-gray-500">Loading journal entries...</p>
              ) : (
                <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
                  {Array.from({ length: daysInMonth }, (_, index) => {
                    const dayDate = new Date(selectedDay.getFullYear(), selectedDay.getMonth(), index + 1);
                    const dayKey = dateKey(dayDate);
                    const entry = entries[dayKey];
                    const isToday = dayKey === dateKey(new Date());
                    return (
                      <button
                        key={dayKey}
                        type="button"
                        onClick={() => {
                          setSelectedDate(dayKey);
                          setIsMonthView(false);
                        }}
                        className={`flex min-h-36 flex-col justify-between rounded-xl border p-4 text-left transition ${
                          isToday
                            ? "border-purple-500/50 bg-purple-500/[0.08] hover:bg-purple-500/[0.13]"
                            : "border-[#2d3038] bg-[#191b20] hover:border-purple-500/35 hover:bg-[#1d1f26]"
                        }`}
                      >
                        <p dir="auto" className={`line-clamp-4 whitespace-pre-wrap text-sm ${entry?.content ? "text-gray-300" : "text-gray-600"}`}>
                          {entry?.content || "No entry for this day"}
                        </p>
                        <span className="mt-5 flex items-center justify-between border-t border-[#30333b] pt-3 text-xs">
                          <span className="flex items-center gap-1.5 text-gray-300">
                            <CalendarDays size={13} className="text-gray-500" />
                            {dateHeadingFormatter.format(dayDate)}
                          </span>
                          <span className="text-purple-300">{weekdayFormatter.format(dayDate)}</span>
                        </span>
                      </button>
                    );
                  })}
                </div>
              )}
            </>
          ) : (
            <div className="mx-auto w-full md:w-[70%]">
              {saveError && currentSaveStatus === "error" && (
                <div role="alert" className="mb-4 flex flex-wrap items-center justify-between gap-3 rounded-xl border border-rose-900/60 bg-rose-950/30 px-4 py-3 text-sm text-rose-200">
                  <span>Could not save this entry: {saveError}</span>
                  <button
                    type="button"
                    onClick={() => void flushEntry(selectedDate).catch(() => undefined)}
                    className="font-semibold underline underline-offset-2"
                  >
                    Retry
                  </button>
                </div>
              )}

              <MarkdownTextEditor
                key={selectedDate}
                value={entryForSelectedDate?.content ?? ""}
                onChange={(content) => updateContent(selectedDate, content)}
                onBlur={handleBlur}
                placeholder="Write about your day... (Markdown supported)"
                direction="auto"
                ariaLabel={`Journal entry for ${selectedDate}`}
                className="vm-journal-markdown-editor vm-journal-entry-editor min-h-[55vh] w-full bg-transparent text-gray-200"
              />
            </div>
          )}
        </div>
      </div>
    </section>
  );
};
