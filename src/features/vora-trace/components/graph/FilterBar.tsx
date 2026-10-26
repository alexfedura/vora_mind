import { useTraceStore } from "../../store/traceStore";
import type { TraceNodeKind, TraceNodeStatus } from "@/types";

const KIND_CHIPS: { label: string; value: TraceNodeKind | "all" }[] = [
  { label: "All", value: "all" },
  { label: "State", value: "state" },
  { label: "Decision", value: "decision" },
  { label: "Experiment", value: "experiment" },
  { label: "Rejected", value: "rejected" },
];

const STATUS_CHIPS: { label: string; value: TraceNodeStatus }[] = [
  { label: "accepted", value: "accepted" },
  { label: "rejected", value: "rejected" },
  { label: "pending", value: "pending" },
  { label: "current", value: "current" },
];

export function FilterBar() {
  const filters = useTraceStore((s) => s.filters);
  const setFilters = useTraceStore((s) => s.setFilters);

  const toggleKind = (k: TraceNodeKind | "all") => {
    if (k === "all") {
      setFilters({ kinds: [] });
      return;
    }
    const cur = filters.kinds;
    setFilters({
      kinds: cur.includes(k) ? cur.filter((x) => x !== k) : [...cur, k],
    });
  };

  const toggleStatus = (s: TraceNodeStatus) => {
    const cur = filters.statuses;
    setFilters({
      statuses: cur.includes(s) ? cur.filter((x) => x !== s) : [...cur, s],
    });
  };

  return (
    <div className="flex items-center gap-3 flex-wrap">
      {/* Search */}
      <input
        type="search"
        placeholder="Search…"
        value={filters.search}
        onChange={(e) => setFilters({ search: e.target.value })}
        className="h-7 w-44 rounded-md border border-trace-border
                   bg-trace-bg px-2 text-xs text-trace-text placeholder-trace-muted
                   outline-none focus:border-trace-accent"
      />

      {/* Kind chips */}
      <div className="flex items-center gap-1">
        {KIND_CHIPS.map((c) => {
          const active =
            c.value === "all"
              ? filters.kinds.length === 0
              : filters.kinds.includes(c.value as TraceNodeKind);
          return (
            <button
              key={c.value}
              onClick={() => toggleKind(c.value)}
              className={`rounded-full px-2.5 py-0.5 text-xs border transition-colors ${
                active
                  ? "border-trace-accent bg-trace-accent/20 text-trace-text"
                  : "border-trace-border bg-trace-bg text-trace-muted hover:border-trace-accent/40"
              }`}
            >
              {c.label}
            </button>
          );
        })}
      </div>

      {/* Status chips */}
      <div className="flex items-center gap-1">
        {STATUS_CHIPS.map((c) => {
          const active = filters.statuses.includes(c.value);
          const color =
            c.value === "accepted" ? "text-emerald-400 border-emerald-500/40" :
            c.value === "rejected" ? "text-red-400 border-red-500/40" :
            c.value === "current"  ? "text-violet-400 border-violet-500/40" :
            "text-trace-muted border-trace-border";
          return (
            <button
              key={c.value}
              onClick={() => toggleStatus(c.value)}
              className={`rounded-full px-2.5 py-0.5 text-xs border transition-colors ${
                active ? `${color} bg-white/5` : "border-trace-border bg-trace-bg text-trace-muted/60 hover:border-trace-border/80"
              }`}
            >
              {c.label}
            </button>
          );
        })}
      </div>

      {/* Clear button when filters active */}
      {(filters.search || filters.kinds.length > 0 || filters.statuses.length > 0) && (
        <button
          onClick={() => setFilters({ search: "", kinds: [], statuses: [] })}
          className="text-xs text-trace-muted hover:text-trace-text"
        >
          clear ×
        </button>
      )}
    </div>
  );
}
